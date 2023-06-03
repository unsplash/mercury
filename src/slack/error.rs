//! Captures what failure can look like when making requests to the Slack API.

use crate::slack::channel::ChannelName;
use std::fmt;

/// Every possible unexceptional fail case when making requests to the Slack API.
pub enum SlackError {
    /// General request failure.
    APIRequestFailed(reqwest::Error),
    /// Successfully decoded response error message.
    APIResponseError(String),
    /// Unable to find the requested channel in our channel <-> id map. It's
    /// possible that the cache is stale.
    UnknownChannel(ChannelName),
}

impl From<reqwest::Error> for SlackError {
    fn from(e: reqwest::Error) -> Self {
        SlackError::APIRequestFailed(e)
    }
}

impl fmt::Display for SlackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = match self {
            SlackError::APIRequestFailed(e) => format!("Slack API request failed: {:?}", e),
            SlackError::APIResponseError(e) => format!("Slack API returned error: {}", e),
            SlackError::UnknownChannel(c) => format!("Unknown Slack channel: {}", c),
        };

        write!(f, "{}", x)
    }
}
