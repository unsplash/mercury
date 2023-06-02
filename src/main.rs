use dotenvy::dotenv;
use std::net::SocketAddr;
use tracing::{info, warn};

pub mod error;
mod router;
pub mod slack;

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
        .expect("PORT environment variable not found")
        .parse()
        .expect("Could not parse PORT to u16");

    let token = std::env::var("SLACK_TOKEN").expect("SLACK_TOKEN environment variable not found");
    slack::auth::TOKEN.set(token).unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Listening on {}", addr.to_string());

    axum::Server::bind(&addr)
        .serve(router::new().into_make_service())
        .await
        .expect("Failed to start server");
}
