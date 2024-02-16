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
    /// From the entity `dyno` (NB *not* `api:dyno`).
    DynoCrash { name: String, status_code: u8 },
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
        HookPayload::Dyno(x) => match is_dyno_crash(x) {
            None => ForwardResult::IgnoredAction,
            Some(status_code) => {
                send(
                    deps,
                    plat,
                    &HookEvent::DynoCrash {
                        name: x.data.name.to_owned(),
                        status_code,
                    },
                    payload,
                )
                .await
            }
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
    let app_name = &get_app_data(payload).name;

    let title = match event {
        HookEvent::Rollback { .. } => format!("🏳️ {}", app_name),
        HookEvent::EnvVarsChange { .. } => format!("⚙️  {}", app_name),
        HookEvent::DynoCrash { .. } => format!("☢️  {}", app_name),
    };

    let desc = match event {
        HookEvent::Rollback { version } => format!("Rollback to {}", version),
        HookEvent::EnvVarsChange { raw_change } => {
            format!("Environment variables changed: {}", raw_change)
        }
        HookEvent::DynoCrash { name, status_code } => {
            format!("Dyno {} crashed with status code {}", name, status_code)
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
/// There's no indication that these descriptions are stable on Heroku's side.
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

/// Determines if a dyno event payload corresponds to a relevant crash, and if
/// so returns the status code.
///
/// This logic is copied from Otto:
/// <https://github.com/unsplash/otto/blob/38c0fc5cf9a0ea5f1443a2fa5f45c0d837ba83a3/app/routes/hooks/monitor.rb#L17>
fn is_dyno_crash(payload: &DynoHookPayload) -> Option<u8> {
    let DynoHookData {
        typ,
        state,
        exit_status,
        ..
    } = &payload.data;

    exit_status.filter(|code| typ != "run" && state == "crashed" && code > &0)
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
    #[serde(rename = "dyno")]
    Dyno(DynoHookPayload),
}

/// The payload supplied by Heroku for the `api:release` entity type.
#[derive(Debug, PartialEq, Deserialize)]
pub struct ReleaseHookPayload {
    data: ReleaseHookData,
    pub action: ReleaseHookAction,
}

/// The payload supplied by Heroku for the `dyno` entity type.
#[derive(Debug, PartialEq, Deserialize)]
pub struct DynoHookPayload {
    data: DynoHookData,
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
    user: UserData,
}

/// General information about an `api:release` webhook event.
#[derive(Debug, PartialEq, Deserialize)]
struct DynoHookData {
    app: AppData,
    name: String,
    #[serde(rename = "type")]
    typ: String,
    state: String,
    /// We need this for `DynoCrash`, however for other types of dyno events it
    /// can be absent or `null`, and we should still serialise those and return
    /// 200.
    exit_status: Option<u8>,
}

/// Common metadata about the app for which a webhook event fired.
#[derive(Debug, PartialEq, Deserialize)]
struct AppData {
    name: String,
}

/// Information about the user who enacted the change.
#[derive(Debug, PartialEq, Deserialize)]
struct UserData {
    email: String,
}

fn get_app_data(payload: &HookPayload) -> &AppData {
    match payload {
        HookPayload::Release(x) => &x.data.app,
        HookPayload::Dyno(x) => &x.data.app,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod deserialization {
        use super::*;

        #[test]
        fn test_root_payload_release() {
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
                        "email": "hodor@unsplash.com"
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
                    user: UserData {
                        email: "hodor@unsplash.com".to_string(),
                    },
                },
                action: ReleaseHookAction::Update,
            });

            assert_eq!(
                expected,
                serde_json::from_str(real_redacted_example).unwrap()
            );
        }

        #[test]
        fn test_root_payload_dyno() {
            let real_redacted_example = r#"{
                "id": "292a769d-53d8-4edd-ace4-017a967653e1",
                "created_at": "2023-08-03T14:19:07Z",
                "data": {
                    "id": "ab9fece7-41cc-4f22-8b73-de8c1a7a52b5",
                    "app": {
                        "id": "b3e4c9d6-3d05-4f2d-98d1-458c358269df",
                        "name": "my-app"
                    },
                    "release": {
                        "id": "3a7a5c18-b1ac-4830-9efb-5b68551e85f0",
                        "version": 7634
                    },
                    "command": "/bin/cowsay moo",
                    "size": "Standard-1X",
                    "exit_status": 137,
                    "management": "run:detached",
                    "state": "crashed",
                    "type": "scheduler",
                    "name": "scheduler.8375"
                },
                "actor": {
                  "id": "1030c06a-bcbe-4738-9134-89af5c717fb1",
                  "email": "noreply+webhooks@heroku.com"
                },
                "previous_data": {},
                "published_at": null,
                "resource": "dyno",
                "action": "destroy",
                "version": "application/vnd.heroku+json; version=3"
            }"#;

            let expected = HookPayload::Dyno(DynoHookPayload {
                data: DynoHookData {
                    app: AppData {
                        name: "my-app".to_string(),
                    },
                    name: "scheduler.8375".to_string(),
                    typ: "scheduler".to_string(),
                    state: "crashed".to_string(),
                    exit_status: Some(137),
                },
            });

            assert_eq!(
                expected,
                serde_json::from_str(real_redacted_example).unwrap()
            );
        }

        #[test]
        fn test_root_payload_dyno_no_status_code() {
            let real_redacted_example = r#"{
                "id": "5c930235-d947-4a8d-aefe-2692604d0c9a",
                "data": {
                    "id": "d0eb9c96-0529-4f9d-bb4c-cc3382a55180",
                    "app": {
                        "id": "b3e4c9d6-3d05-4f2d-98d1-458c358269df",
                        "name": "my-app"
                    },
                    "name": "scheduler.3540",
                    "size": "Standard-1X",
                    "type": "scheduler",
                    "state": "starting",
                    "command": "/bin/cowsay moo",
                    "release": {
                        "id": "9e67f453-d990-4c5f-89d3-c9ca2a70830f",
                        "version": 7636
                    },
                    "attach_url": null,
                    "created_at": "2023-08-03T17:40:49Z",
                    "updated_at": "2023-08-03T17:40:49Z"
                },
                "actor": {
                    "id": "6a65fcb0-507d-45ae-85d9-9bb29fe8d234",
                    "email": "scheduler@addons.heroku.com"
                },
                "action": "create",
                "version": "application/vnd.heroku+json; version=3",
                "resource": "dyno",
                "sequence": null,
                "created_at": "2023-08-03T17:40:49.504132Z",
                "updated_at": "2023-08-03T17:40:49.504139Z",
                "published_at": "2023-08-03T17:40:50Z",
                "previous_data": {},
                "webhook_metadata": {
                    "attempt": {
                        "id": "9c522e4b-46aa-4233-9831-412a69c5357b"
                    },
                    "delivery": {
                        "id": "4505f0a6-9397-4195-8640-33159b48cfb7"
                    },
                    "event": {
                        "id": "5c930235-d947-4a8d-aefe-2692604d0c9a",
                        "include": "api:dyno"
                    },
                    "webhook": {
                        "id": "f7491c4b-2212-46d5-826f-064489daf9c4"
                    }
                }
            }"#;

            let expected = HookPayload::Dyno(DynoHookPayload {
                data: DynoHookData {
                    app: AppData {
                        name: "my-app".to_string(),
                    },
                    name: "scheduler.3540".to_string(),
                    typ: "scheduler".to_string(),
                    state: "starting".to_string(),
                    exit_status: None,
                },
            });

            assert_eq!(
                expected,
                serde_json::from_str(real_redacted_example).unwrap()
            );
        }

        #[test]
        fn test_root_payload_dyno_null_status_code() {
            let synthetic_example = r#"{
                "id": "5c930235-d947-4a8d-aefe-2692604d0c9a",
                "data": {
                    "id": "d0eb9c96-0529-4f9d-bb4c-cc3382a55180",
                    "app": {
                        "id": "b3e4c9d6-3d05-4f2d-98d1-458c358269df",
                        "name": "my-app"
                    },
                    "name": "scheduler.3540",
                    "size": "Standard-1X",
                    "type": "scheduler",
                    "state": "starting",
                    "exit_status": null,
                    "command": "/bin/cowsay moo",
                    "release": {
                        "id": "9e67f453-d990-4c5f-89d3-c9ca2a70830f",
                        "version": 7636
                    },
                    "attach_url": null,
                    "created_at": "2023-08-03T17:40:49Z",
                    "updated_at": "2023-08-03T17:40:49Z"
                },
                "actor": {
                    "id": "6a65fcb0-507d-45ae-85d9-9bb29fe8d234",
                    "email": "scheduler@addons.heroku.com"
                },
                "action": "create",
                "version": "application/vnd.heroku+json; version=3",
                "resource": "dyno",
                "sequence": null,
                "created_at": "2023-08-03T17:40:49.504132Z",
                "updated_at": "2023-08-03T17:40:49.504139Z",
                "published_at": "2023-08-03T17:40:50Z",
                "previous_data": {},
                "webhook_metadata": {
                    "attempt": {
                        "id": "9c522e4b-46aa-4233-9831-412a69c5357b"
                    },
                    "delivery": {
                        "id": "4505f0a6-9397-4195-8640-33159b48cfb7"
                    },
                    "event": {
                        "id": "5c930235-d947-4a8d-aefe-2692604d0c9a",
                        "include": "api:dyno"
                    },
                    "webhook": {
                        "id": "f7491c4b-2212-46d5-826f-064489daf9c4"
                    }
                }
            }"#;

            let expected = HookPayload::Dyno(DynoHookPayload {
                data: DynoHookData {
                    app: AppData {
                        name: "my-app".to_string(),
                    },
                    name: "scheduler.3540".to_string(),
                    typ: "scheduler".to_string(),
                    state: "starting".to_string(),
                    exit_status: None,
                },
            });

            assert_eq!(expected, serde_json::from_str(synthetic_example).unwrap());
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
                    user: UserData {
                        email: "hodor@unsplash.com".to_string(),
                    },
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
