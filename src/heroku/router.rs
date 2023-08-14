//! Heroku subrouter definition.
//!
//! The following subroute is supported:
//!
//! - POST: `/hook`

use super::{auth::*, webhook::*, Platform};
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
/// Accepts a [HookPayload] in `application/json` format. Valid events are
/// forwarded to the specified platform. This feature is potentially
/// temperamental; see [decode_release_payload].
async fn webhook_handler(
    State(deps): State<Deps>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    headers: HeaderMap,
    extract::Query(platform): extract::Query<Platform>,
    // We can't parse this at all yet as we need to compare signatures.
    body_bytes: Bytes,
) -> impl IntoResponse {
    let heroku_secret = deps
        .heroku_secret
        .as_ref()
        .ok_or_else(|| (StatusCode::PRECONDITION_FAILED, String::new()))?;

    if content_type != headers::ContentType::json() {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            String::from("Requests must have `Content-Type: application/json`"),
        ));
    }

    validate_request_signature(heroku_secret, &body_bytes, &headers)
        .await
        .map_err(|e| {
            let msg = match e {
                SecretError::Missing => "Missing Heroku secret",
                SecretError::Invalid => "Invalid Heroku secret",
            };
            warn!(msg);

            (StatusCode::UNAUTHORIZED, String::new())
        })?;

    let payload = serde_json::from_slice::<HookPayload>(&body_bytes).map_err(|e| {
        let msg = format!("Failed to deserialize payload: {}", e);
        warn!(msg);

        (StatusCode::UNPROCESSABLE_ENTITY, msg)
    })?;

    let res = forward(&deps, &platform, &payload).await;

    match res {
        ForwardResult::Failure(ForwardFailure::ToSlack(e)) => Err(handle_slack_err(&e)),
        ForwardResult::UnsupportedEvent(evt) => {
            info!(
                "Could not decode payload to a supported event, found: {}",
                evt
            );

            Ok(())
        }
        ForwardResult::Success | ForwardResult::IgnoredAction => Ok(()),
    }
}
