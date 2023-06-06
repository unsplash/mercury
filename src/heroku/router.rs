//! Heroku subrouter definition.
//!
//! The following route is supported:
//!
//! POST: `/api/v1/heroku/hook`

use axum::{http::StatusCode, response::IntoResponse, routing::post, Router};

/// Instantiate a new Heroku subrouter.
pub fn heroku_router() -> Router {
    Router::new().route("/hook", post(webhook_handler))
}

/// Handler for the POST route `/api/v1/heroku/hook`.
///
/// A `Heroku-Webhook-Hmac-SHA256` header containing the HMAC SHA256 signature
/// of the request body, signed with the shared secret, must be present.
///
/// Accepts a `platform` query param indicating the supported [platform][TODO],
/// along with that platform's respective query params.
async fn webhook_handler() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}
