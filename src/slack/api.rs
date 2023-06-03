//! Type definitions and helpers for the Slack API.

use super::auth::*;
use once_cell::sync::Lazy;
use serde::Deserialize;

/// The base URL of the Slack API.
const API_BASE: &str = "https://slack.com/api";

/// A reusable client that holds a connection pool internally, as per
/// [reqwest::Client].
//`Lazy` allows us to safely reuse the client within this module rather than
// drill it all the way down from the router.
static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

/// Create a GET request to any Slack API endpoint, handling authentication.
pub fn get<T: ToString>(path: T, token: &SlackAccessToken) -> reqwest::RequestBuilder {
    CLIENT
        .get(API_BASE.to_owned() + &path.to_string())
        .header(reqwest::header::AUTHORIZATION, to_auth_header_val(token))
}

/// Create a POST request to any Slack API endpoint, handling authentication.
pub fn post<T: ToString>(path: T, token: &SlackAccessToken) -> reqwest::RequestBuilder {
    CLIENT
        .post(API_BASE.to_owned() + &path.to_string())
        .header(reqwest::header::AUTHORIZATION, to_auth_header_val(token))
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
