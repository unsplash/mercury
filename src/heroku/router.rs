//! Heroku subrouter definition.
//!
//! The following subroute is supported:
//!
//! - POST: `/hook`

use super::{auth::*, platform::Platform, webhook::*};
use crate::{router::Deps, slack::router::handle_slack_err};
use axum::{
    extract::{self, State},
    headers,
    http::{header::HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router, TypedHeader,
};
use hyper::body::Bytes;
use tracing::{info, warn};

/// Instantiate a new Heroku subrouter.
pub fn heroku_router() -> Router<Deps> {
    Router::new().route("/hook", post(webhook_handler))
}

/// Handler for the POST subroute `/hook`.
///
/// A `Heroku-Webhook-Hmac-SHA256` header containing the HMAC SHA256 signature
/// of the request body, signed with the shared secret, must be present.
///
/// Accepts a `platform` query param indicating the supported [Platform], along
/// with that platform's respective query params.
///
/// Accepts a [HookEvent] in `application/json` format. Valid events are
/// forwarded to the specified platform. This feature is potentially
/// temperamental; see [decode_payload].
async fn webhook_handler(
    State(deps): State<Deps>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    headers: HeaderMap,
    extract::Query(platform): extract::Query<Platform>,
    // We can't parse this at all yet as we need to compare signatures.
    body_bytes: Bytes,
) -> impl IntoResponse {
    match &deps.heroku_secret {
        None => (StatusCode::PRECONDITION_FAILED, String::new()),
        Some(heroku_secret) => {
            if content_type != headers::ContentType::json() {
                return (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    String::from("Requests must have `Content-Type: application/json`"),
                );
            }

            let validation = validate_request_signature(heroku_secret, &body_bytes, &headers).await;

            match validation {
                Err(e) => {
                    let msg = match e {
                        SecretError::Missing => "Missing Heroku secret",
                        SecretError::Invalid => "Invalid Heroku secret",
                    };
                    warn!(msg);

                    (StatusCode::UNAUTHORIZED, String::new())
                }
                Ok(()) => {
                    let decoded = serde_json::from_slice::<HookPayload>(&body_bytes);

                    match decoded {
                        Err(e) => {
                            let msg = format!("Failed to deserialize payload: {}", e);
                            warn!(msg);

                            (StatusCode::UNPROCESSABLE_ENTITY, msg)
                        }
                        Ok(payload) => {
                            let res = forward(&deps, &platform, &payload).await;

                            match res {
                                ForwardResult::Success | ForwardResult::IgnoredAction => {
                                    (StatusCode::OK, String::new())
                                }
                                ForwardResult::UnsupportedEvent(evt) => {
                                    info!(
                                        "Could not decode payload to a supported event, found: {}",
                                        evt
                                    );

                                    (StatusCode::OK, String::new())
                                }
                                ForwardResult::Failure(ForwardFailure::ToSlack(e)) => {
                                    handle_slack_err(&e)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
