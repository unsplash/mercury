//! The guide of souls to the underworld.
//!
//! For a high-level introduction see the project README.
//!
//! The only communication mechanism currently supported is [Slack][slack].

use dotenvy::dotenv;
use router::Deps;
use slack::api::{SlackClient, API_BASE};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

mod de;
mod router;
mod slack;

/// Application entrypoint. Initialises tracing, checks for environment
/// variables, binds to 0.0.0.0, and starts the server.
#[tokio::main]
async fn main() {
    // We currently only have tracing (to stdout) for server responses and
    // manual traces. It'd be nice to get tracing for client requests as well:
    //   https://github.com/seanmonstar/reqwest/issues/155
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let has_dotenv = dotenv().is_ok();
    if !has_dotenv {
        warn!("No .env found");
    }

    let port: u16 = std::env::var("PORT")
        .map(|x| x.parse().expect("Could not parse PORT to u16"))
        .unwrap_or(80);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on {}", addr.to_string());

    let slack_client = SlackClient::new(API_BASE.into());
    let deps = Deps {
        slack_client: Arc::new(Mutex::new(slack_client)),
    };

    axum::Server::bind(&addr)
        .serve(router::new(deps).into_make_service())
        .await
        .expect("Failed to start server");
}
