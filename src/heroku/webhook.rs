//! Support a limited set of [webhook events](HookEvent), forwarding them onto a
//! given messaging [Platform].
//!
//! Requests are validated by a secret. See [super::auth].
//!
//! Webhooks must be created externally, supplying Mercury's
//! `/api/v1/heroku/hook` endpoint as the target URL. The `platform` query param
//! determines where messages are sent, along with platform-specific metadata.
//! Events can be filtered by specifying Heroku entity types during webhook
//! creation.
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

/// Supported Heroku webhook events.
#[derive(Debug, PartialEq, Eq)]
pub enum HookEvent {
    /// From the entity `api:release`.
    Rollback { version: String },
    /// From the entity `api:release`.
    EnvVarsChange { raw_change: String },
}

/// The result of attempting to forward a valid webhook.
pub enum ForwardResult {
    IgnoredAction,
    UnsupportedEvent(String),
    Failure(ForwardFailure),
    Success,
}

/// What went wrong during forwarding, specifically in communication with the
/// onward platform.
pub enum ForwardFailure {
    ToSlack(SlackError),
}

/// Validate, filter, and ultimately forward a webhook event to the given
/// [Platform].
pub async fn forward(deps: &Deps, plat: &Platform, payload: &HookPayload) -> ForwardResult {
    match payload {
        HookPayload::Release(x) => match x.action {
            // We only want to send one notification, so we'll
            // ignore anything other than the hopefully lone
            // update action.
            ReleaseHookAction::Other => ForwardResult::IgnoredAction,
            ReleaseHookAction::Update => match decode_release_payload(x) {
                Err(desc) => ForwardResult::UnsupportedEvent(desc),
                Ok(evt) => send(deps, plat, &evt, payload).await,
            },
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
    let app_name = match payload {
        HookPayload::Release(x) => &x.data.app.name,
    };

    let title = match event {
        HookEvent::Rollback { .. } => format!("🏳️ {}", app_name),
        HookEvent::EnvVarsChange { .. } => format!("⚙️  {}", app_name),
    };

    let desc = match event {
        HookEvent::Rollback { version } => format!("Rollback to {}", version),
        HookEvent::EnvVarsChange { raw_change } => {
            format!("Environment variables changed: {}", raw_change)
        }
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
                        avatar: None,
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

/// Attempt to decode a valid webhook payload into a supported [HookEvent].
/// Returns the description that failed decoding upon failure.
///
/// There's no indication that these description are stable on Heroku's side.
pub fn decode_release_payload(payload: &ReleaseHookPayload) -> Result<HookEvent, String> {
    decode_rollback(payload)
        .or_else(|| decode_env_vars_change(payload))
        .ok_or_else(|| payload.data.description.clone())
}

/// Attempt to decode a rollback webhook event from a payload.
fn decode_rollback(payload: &ReleaseHookPayload) -> Option<HookEvent> {
    Regex::new(r"^Rollback to (?P<version>.+)$")
        .ok()
        .and_then(|re| re.captures(&payload.data.description))
        .and_then(|cs| cs.name("version"))
        .map(|m| HookEvent::Rollback {
            version: m.as_str().to_owned(),
        })
}

/// Attempt to decode an environment variable-related webhook event from a
/// payload.
fn decode_env_vars_change(payload: &ReleaseHookPayload) -> Option<HookEvent> {
    Regex::new(r"^(?P<change>.+) config vars$")
        .ok()
        .and_then(|re| re.captures(&payload.data.description))
        .and_then(|cs| cs.name("change"))
        .map(|m| HookEvent::EnvVarsChange {
            raw_change: m.as_str().to_owned(),
        })
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
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "resource")]
pub enum HookPayload {
    #[serde(rename = "release")]
    Release(ReleaseHookPayload),
}

/// The payload supplied by Heroku for the `api:release` entity type.
#[derive(Debug, PartialEq, Deserialize)]
pub struct ReleaseHookPayload {
    data: ReleaseHookData,
    pub action: ReleaseHookAction,
}

/// The action within an `api:release` webhook event lifecycle.
///
/// Multiple payloads can be sent for the same wider event, for example "create"
/// followed by "update".
///
/// <https://help.heroku.com/JP3QR5I5/why-am-i-receiving-2-web-hook-events-for-a-single-release>
#[derive(Debug, PartialEq, Deserialize)]
pub enum ReleaseHookAction {
    #[serde(rename = "update")]
    Update,
    #[serde(other)]
    Other,
}

/// General information about an `api:release` webhook event.
#[derive(Debug, PartialEq, Deserialize)]
struct ReleaseHookData {
    app: AppData,
    description: String,
}

/// Common metadata about the app for which a webhook event fired.
#[derive(Debug, PartialEq, Deserialize)]
struct AppData {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod deserialization {
        use super::*;

        #[test]
        fn test_root_payload() {
            let real_redacted_example = r#"{
                "id": "66a9e685-e1f3-4f9f-9177-a024fb5f0902",
                "data": {
                    "id": "38821f7c-e1a1-41d9-a34b-c41e2fa6d82d",
                    "app": {
                        "id": "59d151db-c38e-4e9c-a854-faead7e8d6cc",
                        "name": "my-app",
                        "process_tier": "production"
                    },
                    "slug": {
                        "id": "507af0a6-a83b-4a16-9a9f-bf55b5864848",
                        "commit": "69eec518969cc409e116940aa5304ab6ab237a4d",
                        "commit_description": ""
                    },
                    "user": {
                        "id": "71def50e-da83-453a-bba3-46b4e26911b0",
                        "email": "hello@example.com"
                    },
                    "stack": "heroku-20",
                    "status": "succeeded",
                    "current": true,
                    "pstable": {
                        "my-process": {
                            "slug": {
                                "id": "fefa649a-845f-4c35-ad9a-2819633b4884"
                            },
                            "command": "/bin/cowsay moo"
                        }
                    },
                    "version": 6644,
                    "created_at": "2023-08-03T10:00:30Z",
                    "updated_at": "2023-08-03T10:00:30Z",
                    "description": "Deploy 69eec518",
                    "addon_plan_names": [],
                    "output_stream_url": null
                },
                "actor": {
                    "id": "71def50e-da83-453a-bba3-46b4e26911b0",
                    "email": "hello@example.com"
                },
                "action": "update",
                "version": "application/vnd.heroku+json; version=3",
                "resource": "release",
                "sequence": null,
                "created_at": "2023-08-03T10:00:30.693808Z",
                "updated_at": "2023-08-03T10:00:30.693817Z",
                "published_at": "2023-08-03T10:00:31Z",
                "previous_data": {},
                "webhook_metadata": {
                    "attempt": {
                        "id": "6ddac576-005d-456c-8153-8fba3f5702de"
                    },
                    "delivery": {
                        "id": "ecb8532f-3b9b-4f8a-8eee-14296e0a6784"
                    },
                    "event": {
                        "id": "66a9e685-e1f3-4f9f-9177-a024fb5f0902",
                        "include": "api:release"
                    },
                    "webhook": {
                        "id": "af83d062-fdfe-4fc0-88ad-b91bf58f0656"
                    }
                }
            }"#;

            let expected = HookPayload::Release(ReleaseHookPayload {
                data: ReleaseHookData {
                    app: AppData {
                        name: "my-app".to_string(),
                    },
                    description: "Deploy 69eec518".to_string(),
                },
                action: ReleaseHookAction::Update,
            });

            assert_eq!(
                expected,
                serde_json::from_str(real_redacted_example).unwrap()
            );
        }
    }

    mod decode_payload {
        use super::*;

        fn payload_from_desc<T: ToString>(desc: T) -> ReleaseHookPayload {
            ReleaseHookPayload {
                data: ReleaseHookData {
                    app: AppData {
                        name: "any".to_string(),
                    },
                    description: desc.to_string(),
                },
                action: ReleaseHookAction::Update,
            }
        }

        #[test]
        fn test_rollback() {
            assert_eq!(
                decode_release_payload(&payload_from_desc("Rollback to v1234")),
                Ok(HookEvent::Rollback {
                    version: "v1234".to_string()
                }),
            );

            assert_eq!(
                decode_release_payload(&payload_from_desc("Rollback to some new format")),
                Ok(HookEvent::Rollback {
                    version: "some new format".to_string()
                }),
            );

            assert_eq!(
                decode_release_payload(&payload_from_desc("rolled back to v1234")),
                Err("rolled back to v1234".to_string()),
            );
        }

        #[test]
        fn test_env_vars_change() {
            assert_eq!(
                decode_release_payload(&payload_from_desc("Set FOO, BAR config vars")),
                Ok(HookEvent::EnvVarsChange {
                    raw_change: "Set FOO, BAR".to_string()
                }),
            );

            assert_eq!(
                decode_release_payload(&payload_from_desc("Some new format config vars")),
                Ok(HookEvent::EnvVarsChange {
                    raw_change: "Some new format".to_string()
                }),
            );

            assert_eq!(
                decode_release_payload(&payload_from_desc("Config vars changed")),
                Err("Config vars changed".to_string()),
            );
        }
    }
}
