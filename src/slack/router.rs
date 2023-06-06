//! Slack subrouter definition.
//!
//! The following subroute is supported:
//!
//! POST: `/`

use crate::slack::{api::SlackClient, auth::SlackAccessToken, error::SlackError, message::Message};
use axum::{
    extract::{self, State},
    headers,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router, TypedHeader,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

/// Instantiate a new Slack subrouter.
pub fn slack_router(slack_client: Arc<Mutex<SlackClient>>) -> Router {
    Router::new()
        .route("/", post(msg_handler))
        .with_state(slack_client)
}

/// Handler for the POST subroute `/`.
///
/// A `Bearer` `Authorization` header containing a Slack access token must be
/// present.
///
/// Accepts a [Message] in `x-www-form-urlencoded` format.
async fn msg_handler(
    State(slack_client): State<Arc<Mutex<SlackClient>>>,
    TypedHeader(t): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    extract::Form(m): extract::Form<Message>,
) -> impl IntoResponse {
    let res = slack_client
        .lock()
        .await
        .post_message(&m, &SlackAccessToken(t.token().into()))
        .await;

    match res {
        Ok(_) => (StatusCode::OK, String::new()),
        Err(e) => {
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
    }
}

/// Parse Slack's API response error to determine if the issue is that the
/// access token failed to provide authentication.
fn is_unauthenticated(res: &SlackError) -> bool {
    match res {
        SlackError::APIResponseError(e) => e == "invalid_auth",
        _ => false,
    }
}
