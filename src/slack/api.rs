use super::{auth::*, error::SlackError};
use once_cell::sync::Lazy;

const API_BASE: &str = "https://slack.com/api";

/// A reusable client that holds a connection pool internally, as per
/// <reqwest::Client>. `Lazy` allows us to safely reuse the client within this
/// module rather than drill it all the way down from the router.
static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

/// Create a GET request to the specified `path` endpoint, handling
/// authentication.
pub fn get<T: ToString>(path: T, token: &SlackAccessToken) -> reqwest::RequestBuilder {
    CLIENT
        .get(API_BASE.to_owned() + &path.to_string())
        .header(reqwest::header::AUTHORIZATION, to_auth_header_val(token))
}

/// Create a POST request to the specified `path` endpoint, handling
/// authentication.
pub fn post<T: ToString>(path: T, token: &SlackAccessToken) -> reqwest::RequestBuilder {
    CLIENT
        .post(API_BASE.to_owned() + &path.to_string())
        .header(reqwest::header::AUTHORIZATION, to_auth_header_val(token))
}

/// All the Slack API calls we use include an optional `error` key.
pub fn decode_error(me: Option<String>) -> SlackError {
    match me {
        None => SlackError::APIResponseMissingError,
        Some(e) => SlackError::APIResponseError(e),
    }
}
