use super::auth::TOKEN;
use crate::error::Failure;
use reqwest;

const API_BASE: &'static str = "https://slack.com/api";

/// Create a GET request to the specified `path` endpoint, handling
/// authentication.
pub fn get<T: ToString>(path: T) -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .get(API_BASE.to_owned() + &path.to_string())
        .header(reqwest::header::AUTHORIZATION, "Bearer ".to_owned() + TOKEN)
}

/// Create a POST request to the specified `path` endpoint, handling
/// authentication.
pub fn post<T: ToString>(path: T) -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .post(API_BASE.to_owned() + &path.to_string())
        .header(reqwest::header::AUTHORIZATION, "Bearer ".to_owned() + TOKEN)
}

/// All the Slack API calls we use include an optional `error` key.
pub fn decode_error(me: Option<String>) -> Failure {
    match me {
        None => Failure::SlackAPIResponseMissingError,
        Some(e) => Failure::SlackAPIResponseError(e),
    }
}
