use crate::{
    error::Failure,
    slack::message::{post_message, Message},
};
use axum::{extract, http::StatusCode, routing::post, Router};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

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
async fn slack_handler(extract::Form(m): extract::Form<Message>) -> (StatusCode, String) {
    let res = post_message(&m).await;

    match res {
        Ok(_) => (StatusCode::OK, String::new()),
        Err(e) => {
            let code = match &e {
                Failure::SlackAPIRequestFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
                Failure::SlackAPIResponseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                Failure::SlackAPIResponseMissingError => StatusCode::BAD_GATEWAY,
                Failure::SlackUnknownChannel(_) => StatusCode::BAD_REQUEST,
            };

            (code, e.to_string())
        }
    }
}
