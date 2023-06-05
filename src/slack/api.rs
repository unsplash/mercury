//! Type definitions and helpers for the Slack API.

use super::{auth::*, channel::ChannelMap};
use serde::Deserialize;

/// The base URL of the Slack API.
pub const API_BASE: &str = "https://slack.com/api";

/// Holds a client request pool and a channel map against a base URL.
pub struct SlackClient {
    client: reqwest::Client,
    base_url: String,
    pub(super) channel_map: Option<ChannelMap>,
}

impl SlackClient {
    /// Instantiate against a given base URL, enabling easy mocking. For
    /// real-world usage see [API_BASE].
    pub fn new(base_url: String) -> Self {
        SlackClient {
            client: reqwest::Client::new(),
            base_url,
            channel_map: None,
        }
    }

    /// Create a GET request to any Slack API endpoint, handling authentication.
    pub fn get<T: ToString>(&self, path: T, token: &SlackAccessToken) -> reqwest::RequestBuilder {
        self.client
            .get(self.base_url.clone() + &path.to_string())
            .header(reqwest::header::AUTHORIZATION, to_auth_header_val(token))
    }

    /// Create a POST request to any Slack API endpoint, handling authentication.
    pub fn post<T: ToString>(&self, path: T, token: &SlackAccessToken) -> reqwest::RequestBuilder {
        self.client
            .post(self.base_url.clone() + &path.to_string())
            .header(reqwest::header::AUTHORIZATION, to_auth_header_val(token))
    }
}

/// Slack's API returns a common "untagged" response, representing whether a
/// request was successful.
///
/// ```json
/// {
///     "ok": true,
///     "channels": []
/// }
/// ```
///
/// ```json
/// {
///     "ok": false,
///     "error": "invalid_auth"
/// }
/// ```
#[derive(Deserialize)]
#[serde(untagged)]
pub enum APIResult<T> {
    Ok(T),
    Err(ErrorResponse),
}

/// The universal response in case of an unsuccessful request.
// The `ok` field is checked here, and should be checked on responses too,
// primarily to ensure appropriate deserialization behaviour in case of an
// otherwise empty successful response.
//
// Ideally we'd be able to use `ok` as a tag, rather than defining `APIResult`
// as untagged. See:
//   <https://github.com/serde-rs/serde/issues/745#issuecomment-294314786>
#[derive(Deserialize)]
pub struct ErrorResponse {
    #[allow(dead_code)]
    #[serde(deserialize_with = "crate::de::only_false")]
    ok: bool,
    pub error: String,
}
