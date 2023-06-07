//! Server router definition.
//!
//! The following routes are supported:
//!
//! - GET: `/api/v1/health`
//! - POST: `/api/v1/slack`
//! - POST: `/api/v1/heroku/hook`

use crate::{
    heroku::{auth::HerokuSecret, router::heroku_router},
    slack::{api::SlackClient, auth::SlackAccessToken, router::slack_router},
};
use axum::{http::StatusCode, routing::get, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

/// Dependencies shared by routes across requests.
#[derive(Clone)]
pub struct Deps {
    pub slack_client: Arc<Mutex<SlackClient>>,
    pub slack_token: SlackAccessToken,
    pub heroku_secret: Option<HerokuSecret>,
}

/// Instantiate a new router with tracing.
pub fn new(deps: Deps) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO));

    let v1 = Router::new()
        .nest("/slack", slack_router(&deps.slack_token))
        .nest("/heroku", heroku_router())
        .with_state(deps)
        .layer(trace_layer)
        // Exclude the health check route from tracing.
        .route("/health", get(|| async { StatusCode::OK }));

    let api = Router::new().nest("/v1", v1);

    Router::new().nest("/api", api)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use mockito::Matcher;
    use tower::{Service, ServiceExt};

    fn router(
        base_slack_url: String,
        slack_token: SlackAccessToken,
        heroku_secret: Option<HerokuSecret>,
    ) -> Router {
        super::new(Deps {
            slack_client: Arc::new(Mutex::new(SlackClient::new(base_slack_url))),
            slack_token,
            heroku_secret,
        })
    }

    fn router_() -> Router {
        router(
            "any".to_owned(),
            SlackAccessToken("foobar".to_owned()),
            Some(HerokuSecret("foobarbaz".to_owned())),
        )
    }

    async fn server() -> mockito::ServerGuard {
        mockito::Server::new_async().await
    }

    async fn plaintext_body(body: axum::body::BoxBody) -> String {
        let bytes = hyper::body::to_bytes(body).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    mod general {
        use super::*;

        #[tokio::test]
        async fn test_not_found() {
            let req = Request::builder()
                .uri("/bad/route")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::NOT_FOUND);
        }
    }

    mod slack {
        use super::*;

        #[tokio::test]
        async fn test_not_found() {
            let req = Request::builder()
                .uri("/api/v1/slack/oops")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_bad_method() {
            let req = Request::builder()
                .method("GET")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
        }

        #[tokio::test]
        async fn test_missing_auth() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_bad_content_type() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/xml")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Form requests must have `Content-Type: application/x-www-form-urlencoded`"
            );
        }

        #[tokio::test]
        async fn test_missing_field() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Failed to deserialize form body: missing field `title`"
            );
        }

        #[tokio::test]
        async fn test_bad_auth_for_mercury() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("title".to_owned(), "a title".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let res = router(
                "any".to_owned(),
                SlackAccessToken("not foobar".to_owned()),
                None,
            )
            .oneshot(req)
            .await
            .unwrap();

            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_bad_auth_for_slack() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("title".to_owned(), "a title".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let list_res = r#"{
                "ok": false,
                "error": "invalid_auth"
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let res = router(srv.url(), SlackAccessToken("foobar".to_owned()), None)
                .oneshot(req)
                .await
                .unwrap();

            list_mock.assert_async().await;

            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Slack API returned error: invalid_auth"
            );
        }

        #[tokio::test]
        async fn test_bad_channel() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("title".to_owned(), "a title".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let list_res = r#"{
                "ok": true,
                "channels": [],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let res = router(srv.url(), SlackAccessToken("foobar".to_owned()), None)
                .oneshot(req)
                .await
                .unwrap();

            list_mock.assert_async().await;

            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Unknown Slack channel: channel-name"
            );
        }

        #[tokio::test]
        async fn test_success_without_join() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("title".to_owned(), "a title".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let list_res = r#"{
                "ok": true,
                "channels": [{
                    "id": "channel-id",
                    "name": "channel-name"
                }],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#;

            let msg_res = r#"{
                "ok": true
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let msg_mock = srv
                .mock("POST", "/chat.postMessage")
                .with_body(msg_res)
                .create_async()
                .await;

            let res = router(srv.url(), SlackAccessToken("foobar".to_owned()), None)
                .oneshot(req)
                .await
                .unwrap();

            list_mock.assert_async().await;
            msg_mock.assert_async().await;

            assert_eq!(res.status(), StatusCode::OK);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_success_with_join() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("title".to_owned(), "a title".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let list_res = r#"{
                "ok": true,
                "channels": [{
                    "id": "channel-id",
                    "name": "channel-name"
                }],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#;

            let msg1_res = r#"{
                "ok": false,
                "error": "not_in_channel"
            }"#;

            let join_res = r#"{
                "ok": true
            }"#;

            let msg2_res = r#"{
                "ok": true
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let msg1_mock = srv
                .mock("POST", "/chat.postMessage")
                .with_body(msg1_res)
                .create_async()
                .await;

            let join_mock = srv
                .mock("POST", "/conversations.join")
                .with_body(join_res)
                .create_async()
                .await;

            let msg2_mock = srv
                .mock("POST", "/chat.postMessage")
                .with_body(msg2_res)
                .create_async()
                .await;

            let res = router(srv.url(), SlackAccessToken("foobar".to_owned()), None)
                .oneshot(req)
                .await
                .unwrap();

            list_mock.assert_async().await;
            msg1_mock.assert_async().await;
            join_mock.assert_async().await;
            msg2_mock.assert_async().await;

            assert_eq!(res.status(), StatusCode::OK);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_success_cached_channel() {
            let fields = &[
                ("channel".to_owned(), "channel-name".to_owned()),
                ("title".to_owned(), "a title".to_owned()),
                ("desc".to_owned(), "a description".to_owned()),
            ];
            let msg = serde_urlencoded::to_string(fields).unwrap();

            let req1 = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg.clone()))
                .unwrap();

            let req2 = Request::builder()
                .method("POST")
                .uri("/api/v1/slack")
                .header("Authorization", "Bearer foobar")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(msg))
                .unwrap();

            let list_res = r#"{
                "ok": true,
                "channels": [{
                    "id": "channel-id",
                    "name": "channel-name"
                }],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#;

            let msg1_res = r#"{
                "ok": true
            }"#;

            let msg2_res = r#"{
                "ok": true
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let msg1_mock = srv
                .mock("POST", "/chat.postMessage")
                .with_body(msg1_res)
                .create_async()
                .await;

            let msg2_mock = srv
                .mock("POST", "/chat.postMessage")
                .with_body(msg2_res)
                .create_async()
                .await;

            let mut rt = router(srv.url(), SlackAccessToken("foobar".to_owned()), None);
            let res1 = rt.call(req1).await.unwrap();
            let res2 = rt.call(req2).await.unwrap();

            list_mock.assert_async().await;
            msg1_mock.assert_async().await;
            msg2_mock.assert_async().await;

            assert_eq!(res1.status(), StatusCode::OK);
            assert!(plaintext_body(res1.into_body()).await.is_empty());

            assert_eq!(res2.status(), StatusCode::OK);
            assert!(plaintext_body(res2.into_body()).await.is_empty());
        }
    }

    mod heroku {
        use super::*;

        #[tokio::test]
        async fn test_not_found() {
            let req = Request::builder()
                .uri("/api/v1/heroku/oops")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::NOT_FOUND);
        }

        #[tokio::test]
        async fn test_bad_method() {
            let req = Request::builder()
                .method("GET")
                .uri("/api/v1/heroku/hook")
                .header("Authorization", "Bearer foobar")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
        }

        #[tokio::test]
        async fn test_missing_content_type() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Header of type `content-type` was missing"
            );
        }

        #[tokio::test]
        async fn test_bad_content_type() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Content-Type", "application/xml")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_missing_query() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Failed to deserialize query string: missing field `platform`"
            );
        }

        #[tokio::test]
        async fn test_bad_query() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=discord")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                plaintext_body(res.into_body()).await,
                "Failed to deserialize query string: unknown variant `discord`, expected `slack`"
            );
        }

        #[tokio::test]
        async fn test_missing_signature() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "any": true }"#))
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_bad_signature() {
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Heroku-Webhook-Hmac-SHA256", "bad signature")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{ "any": true }"#))
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_bad_field() {
            let payload = r#"{
                "data": {
                    "app": {
                        "name": false
                    },
                    "description": "any"
                }
            }"#;
            let sig = "tfahlLWwDM4yf8J6r1m+w/fQHMqG7EPNLeXephkimqg=";

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Heroku-Webhook-Hmac-SHA256", sig)
                .header("Content-Type", "application/json")
                .body(Body::from(payload))
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_unsupported_event() {
            let payload = r#"{
                "data": {
                    "app": {
                        "name": "any"
                    },
                    "description": "any"
                },
                "action": "update"
            }"#;
            let sig = "uZ1HWiOtMx9go0dKuGnGGLlEpJNZT/r0tLG9XM3ojG4=";

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Heroku-Webhook-Hmac-SHA256", sig)
                .header("Content-Type", "application/json")
                .body(Body::from(payload))
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::OK);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_ignored_action() {
            let payload = r#"{
                "data": {
                    "app": {
                        "name": "any"
                    },
                    "description": "any"
                },
                "action": "create"
            }"#;
            let sig = "JDWBStTUgQ1SK9JbyCgkAo9KrAcV6BHOZTc4R1IdPjc=";

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Heroku-Webhook-Hmac-SHA256", sig)
                .header("Content-Type", "application/json")
                .body(Body::from(payload))
                .unwrap();

            let res = router_().oneshot(req).await.unwrap();

            assert_eq!(res.status(), StatusCode::OK);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_slack_failure() {
            let payload = r#"{
                "data": {
                    "app": {
                        "name": "any"
                    },
                    "description": "Rollback to v1234"
                },
                "action": "update"
            }"#;
            let sig = "pahzDFn5oWAMM2YMCycs+vFo9JTRIUmfsnuzgM9HXJM=";

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=foo")
                .header("Heroku-Webhook-Hmac-SHA256", sig)
                .header("Content-Type", "application/json")
                .body(Body::from(payload))
                .unwrap();

            let list_res = r#"{
                "ok": false,
                "error": "invalid_auth"
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let res = router(
                srv.url(),
                SlackAccessToken("foobar".to_owned()),
                Some(HerokuSecret("foobarbaz".to_owned())),
            )
            .oneshot(req)
            .await
            .unwrap();

            list_mock.assert_async().await;

            assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }

        #[tokio::test]
        async fn test_slack_success() {
            let payload = r#"{
                "data": {
                    "app": {
                        "name": "any"
                    },
                    "description": "Rollback to v1234"
                },
                "action": "update"
            }"#;
            let sig = "pahzDFn5oWAMM2YMCycs+vFo9JTRIUmfsnuzgM9HXJM=";

            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/heroku/hook?platform=slack&channel=channel-name")
                .header("Heroku-Webhook-Hmac-SHA256", sig)
                .header("Content-Type", "application/json")
                .body(Body::from(payload))
                .unwrap();

            let list_res = r#"{
                "ok": true,
                "channels": [{
                    "id": "channel-id",
                    "name": "channel-name"
                }],
                "response_metadata": {
                    "next_cursor": ""
                }
            }"#;

            let msg_res = r#"{
                "ok": true
            }"#;

            let mut srv = server().await;

            let list_mock = srv
                .mock("GET", "/conversations.list")
                .match_query(Matcher::Any)
                .with_body(list_res)
                .create_async()
                .await;

            let msg_mock = srv
                .mock("POST", "/chat.postMessage")
                .with_body(msg_res)
                .create_async()
                .await;

            let res = router(
                srv.url(),
                SlackAccessToken("foobar".to_owned()),
                Some(HerokuSecret("foobarbaz".to_owned())),
            )
            .oneshot(req)
            .await
            .unwrap();

            list_mock.assert_async().await;
            msg_mock.assert_async().await;

            assert_eq!(res.status(), StatusCode::OK);
            assert!(plaintext_body(res.into_body()).await.is_empty());
        }
    }
}
