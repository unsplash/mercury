//! The guide of souls to the underworld.
//!
//! For a high-level introduction see the project README.
//!
//! The only communication mechanism currently supported is [Slack][slack].

use dotenvy::dotenv;
use heroku::auth::HerokuSecret;
use router::Deps;
use slack::api::{SlackClient, API_BASE};
use std::sync::Arc;
use std::{env, net::SocketAddr};
use tokio::sync::{oneshot, Mutex};
use tracing::{info, warn};

mod de;
mod heroku;
mod router;
mod slack;

/// Application entrypoint. Initialises tracing, checks for environment
/// variables, binds to 0.0.0.0, and starts the server.
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let has_dotenv = dotenv().is_ok();
    if !has_dotenv {
        warn!("No .env found");
    }

    let port: u16 = env::var("PORT")
        .map(|x| x.parse().expect("Could not parse PORT to u16"))
        .unwrap_or(80);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    server_(addr).await;
}

/// Initialise a server without graceful shutdown.
async fn server_(addr: SocketAddr) {
    // Giving a receiver that will never resolve.
    server(addr, oneshot::channel::<()>().1).await;
}

/// Initialise a server with graceful shutdown via `rx`.
async fn server(addr: SocketAddr, rx: oneshot::Receiver<()>) {
    info!("Listening on {}", addr.to_string());

    let heroku_secret = env::var("HEROKU_SECRET").ok().map(HerokuSecret);
    if heroku_secret.is_none() {
        warn!("No $HEROKU_SECRET environment variable found");
    }

    let slack_client = SlackClient::new(API_BASE.into());
    let deps = Deps {
        slack_client: Arc::new(Mutex::new(slack_client)),
        heroku_secret,
    };

    axum::Server::bind(&addr)
        .serve(router::new(deps).into_make_service())
        .with_graceful_shutdown(async {
            rx.await.ok();
        })
        .await
        .expect("Failed to start server");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_real_health_api() {
        let (tx, rx) = oneshot::channel::<()>();

        // Port 0 requests that the OS assigns us an available port.
        let addr = std::net::TcpListener::bind("0.0.0.0:0")
            .unwrap()
            .local_addr()
            .unwrap();

        // Move the server into the background so that it's not blocking.
        tokio::spawn(async move { server(addr, rx).await });

        let res = reqwest::Client::new()
            .get(format!("http://localhost:{}/api/v1/health", addr.port()))
            .send()
            .await
            .unwrap();

        tx.send(()).unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert!(res.text().await.unwrap().is_empty());
    }
}
