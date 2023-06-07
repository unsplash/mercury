//! Support a limited set of webhook events, forwarding them onto a given
//! messaging [Platform].
//!
//! Requests are validated by a secret. See [super::auth].
//!
//! Webhooks must be created externally, supplying Mercury's
//! `/api/v1/heroku/hook` endpoint as the target URL. The `platform` query param
//! determines where messages are sent, along with platform-specific metadata.
//! The webhook entity with the relevant events is `api:release`.
//!
//! Currently the only supported platform is [Slack][slack], which takes
//! an additional `channel` query param (as per
//! [SlackPlatform][super::platform::slack::SlackPlatform]), for example
//! `/api/v1/heroku/hook?platform=slack&channel=playground`. The message
//! structure is fixed.

use super::{dashboard::activity_page_url, Platform};
use crate::{
    router::Deps,
    slack::{self, SlackError},
};
use regex::Regex;
use serde::Deserialize;

pub enum ForwardResult {
    IgnoredAction,
    UnsupportedEvent(String),
    Failure(ForwardFailure),
    Success,
}

pub enum ForwardFailure {
    ToSlack(SlackError),
}

/// Validate, filter, and ultimately forward a webhook event to the given
/// [Platform].
pub async fn forward(deps: &Deps, plat: &Platform, payload: &HookPayload) -> ForwardResult {
    match payload.action {
        // We only want to send one notification, so we'll
        // ignore anything other than the hopefully lone
        // update action.
        HookAction::Other => ForwardResult::IgnoredAction,
        HookAction::Update => match decode_payload(payload) {
            Err(desc) => ForwardResult::UnsupportedEvent(desc),
            Ok(evt) => send(deps, plat, &evt, payload).await,
        },
    }
}

/// Send a valid webhook event to the given [Platform].
async fn send(
    deps: &Deps,
    plat: &Platform,
    event: &HookEvent,
    payload: &HookPayload,
) -> ForwardResult {
    let app_name = &payload.data.app.name;

    let title = match event {
        HookEvent::Rollback(_) => format!("Rollback on {}", app_name),
        HookEvent::EnvVarsChange(_) => {
            format!("Environment variables changed on {}", app_name)
        }
    };

    let desc = match event {
        HookEvent::Rollback(v) => format!("To {}", v),
        HookEvent::EnvVarsChange(evs) => evs.to_string(),
    };

    match plat {
        Platform::Slack(x) => {
            let res = deps
                .slack_client
                .lock()
                .await
                .post_message(
                    &slack::Message {
                        channel: x.channel.clone(),
                        title,
                        desc,
                        link: Some(activity_page_url(app_name)),
                        cc: None,
                    },
                    &deps.slack_token,
                )
                .await;

            match res {
                Err(e) => ForwardResult::Failure(ForwardFailure::ToSlack(e)),
                Ok(_) => ForwardResult::Success,
            }
        }
    }
}

/// Supported Heroku webhook events.
#[derive(Debug, PartialEq, Eq)]
pub enum HookEvent {
    Rollback(String),
    EnvVarsChange(String),
}

/// Attempt to decode a valid webhook payload into a supported [HookEvent].
/// Returns the description that failed decoding upon failure.
///
/// There's no indication that these description are stable on Heroku's side.
pub fn decode_payload(payload: &HookPayload) -> Result<HookEvent, String> {
    decode_rollback(payload)
        .or_else(|| decode_env_vars_change(payload))
        .ok_or_else(|| payload.data.description.clone())
}

/// Attempt to decode a rollback webhook event from a payload.
fn decode_rollback(payload: &HookPayload) -> Option<HookEvent> {
    Regex::new(r"^Rollback to (?P<version>.+)$")
        .ok()
        .and_then(|re| re.captures(&payload.data.description))
        .and_then(|cs| cs.name("version"))
        .map(|m| HookEvent::Rollback(m.as_str().to_owned()))
}

/// Attempt to decode an environment variable-related webhook event from a
/// payload.
fn decode_env_vars_change(payload: &HookPayload) -> Option<HookEvent> {
    Regex::new(r"^(?P<change>.+) config vars$")
        .ok()
        .and_then(|re| re.captures(&payload.data.description))
        .and_then(|cs| cs.name("change"))
        .map(|m| HookEvent::EnvVarsChange(m.as_str().to_owned()))
}

/// The anticipated payload supplied by Heroku in webhook requests.
///
/// This isn't very well documented. An example request is provided here:
///
/// <https://devcenter.heroku.com/articles/app-webhooks#receiving-webhooks>
///
/// Real payloads from a given Heroku app's webhooks can be found here:
///
/// <https://dashboard.heroku.com/apps/HEROKU_APP/webhooks/>
#[derive(Deserialize)]
pub struct HookPayload {
    data: HookData,
    pub action: HookAction,
}

/// The action within a webhook event lifecycle.
///
/// Multiple payloads can be sent for the same wider event, for example "create"
/// followed by "update".
///
/// <https://help.heroku.com/JP3QR5I5/why-am-i-receiving-2-web-hook-events-for-a-single-release>
#[derive(Deserialize)]
pub enum HookAction {
    #[serde(rename = "update")]
    Update,
    #[serde(other)]
    Other,
}

/// General information about a webhook event delivered in [HookPayload].
#[derive(Deserialize)]
struct HookData {
    app: AppData,
    description: String,
}

/// Metadata about the app for which a webhook event fired in [HookData].
#[derive(Deserialize)]
struct AppData {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decode_payload {
        use super::*;

        fn payload_from_desc<T: ToString>(desc: T) -> HookPayload {
            HookPayload {
                data: HookData {
                    app: AppData {
                        name: "any".to_string(),
                    },
                    description: desc.to_string(),
                },
                action: HookAction::Update,
            }
        }

        #[test]
        fn test_rollback() {
            assert_eq!(
                decode_payload(&payload_from_desc("Rollback to v1234")),
                Ok(HookEvent::Rollback("v1234".to_string())),
            );

            assert_eq!(
                decode_payload(&payload_from_desc("Rollback to some new format")),
                Ok(HookEvent::Rollback("some new format".to_string())),
            );

            assert_eq!(
                decode_payload(&payload_from_desc("rolled back to v1234")),
                Err("rolled back to v1234".to_string()),
            );
        }

        #[test]
        fn test_env_vars_change() {
            assert_eq!(
                decode_payload(&payload_from_desc("Set FOO, BAR config vars")),
                Ok(HookEvent::EnvVarsChange("Set FOO, BAR".to_string())),
            );

            assert_eq!(
                decode_payload(&payload_from_desc("Some new format config vars")),
                Ok(HookEvent::EnvVarsChange("Some new format".to_string())),
            );

            assert_eq!(
                decode_payload(&payload_from_desc("Config vars changed")),
                Err("Config vars changed".to_string()),
            );
        }
    }
}
