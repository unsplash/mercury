use crate::slack::channel::ChannelName;
use std::fmt;

/// Sum type representing every possible unexceptional fail state.
pub enum SlackError {
    APIRequestFailed(reqwest::Error),
    APIResponseError(String),
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
