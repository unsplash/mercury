//! Slack subrouter definition.
//!
//! The following subroute is supported:
//!
//! - POST: `/`

use crate::{
    router::Deps,
    slack::{auth::SlackAccessToken, error::SlackError, message::Message},
};
use axum::{
    extract::{self, State},
    headers,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router, TypedHeader,
};
use tower_http::validate_request::ValidateRequestHeaderLayer;
use tracing::error;

/// Instantiate a new Slack subrouter.
pub fn slack_router(slack_token: &SlackAccessToken) -> Router<Deps> {
    Router::new()
        .route("/", post(msg_handler))
        // Unsure how to access `Deps` here to obviate the need for the function
        // parameter.
        .layer(ValidateRequestHeaderLayer::bearer(&slack_token.0))
}

/// Handler for the POST subroute `/`.
///
/// A `Bearer` `Authorization` header containing a Slack access token must be
/// present and must match that found in `$SLACK_TOKEN`.
///
/// Accepts a [Message] in `application/x-www-form-urlencoded` format.
async fn msg_handler(
    State(deps): State<Deps>,
    TypedHeader(t): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    extract::Form(m): extract::Form<Message>,
) -> impl IntoResponse {
    let res = deps
        .slack_client
        .lock()
        .await
        .post_message(&m, &SlackAccessToken(t.token().into()))
        .await;

    match res {
        Ok(_) => (StatusCode::OK, String::new()),
        Err(e) => handle_slack_err(&e),
    }
}

pub fn handle_slack_err(e: &SlackError) -> (StatusCode, String) {
    let code = match &e {
        e if is_unauthenticated(e) => StatusCode::UNAUTHORIZED,
        SlackError::APIRequestFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        SlackError::APIResponseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        SlackError::UnknownChannel(_) => StatusCode::BAD_REQUEST,
    };

    let es = e.to_string();

    error!(es);
    (code, es)
}

/// Parse Slack's API response error to determine if the issue is that the
/// access token failed to provide authentication.
fn is_unauthenticated(res: &SlackError) -> bool {
    match res {
        SlackError::APIResponseError(e) => e == "invalid_auth",
        _ => false,
    }
}
