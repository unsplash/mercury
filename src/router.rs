use crate::slack::{
    auth::SlackAccessToken,
    error::SlackError,
    message::{post_message, Message},
};
use axum::{extract, headers, http::StatusCode, routing::post, Router, TypedHeader};
use tower_http::trace::{self, TraceLayer};
use tracing::{error, Level};

pub fn new() -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO));

    // We only have a single route for now, but this design is extensible.
    //
    // Axum implicitly takes care of all unhappy paths.
    let v1 = Router::new().route("/slack", post(slack_handler));
    let api = Router::new().nest("/v1", v1);
    Router::new().nest("/api", api).layer(trace_layer)
}

// Currently this only supports form bodies. For JSON as well there'll be some
// boilerplate, see:
//   https://github.com/tokio-rs/axum/issues/1654
async fn slack_handler(
    TypedHeader(t): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    extract::Form(m): extract::Form<Message>,
) -> (StatusCode, String) {
    let res = post_message(&m, &SlackAccessToken(t.token().into())).await;

    match res {
        Ok(_) => (StatusCode::OK, String::new()),
        Err(e) => {
            let code = match &e {
                SlackError::APIRequestFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
                SlackError::APIResponseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                SlackError::APIResponseMissingError => StatusCode::BAD_GATEWAY,
                SlackError::UnknownChannel(_) => StatusCode::BAD_REQUEST,
            };

            let es = e.to_string();

            error!(es);
            (code, es)
        }
    }
}
