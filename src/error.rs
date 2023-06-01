use crate::slack::channel::ChannelName;
use std::fmt;

/// Sum type representing every possible unexceptional fail state.
pub enum Failure {
    SlackAPIRequestFailed(reqwest::Error),
    SlackAPIResponseError(String),
    SlackAPIResponseMissingError,
    SlackUnknownChannel(ChannelName),
}

impl From<reqwest::Error> for Failure {
    fn from(e: reqwest::Error) -> Self {
        Failure::SlackAPIRequestFailed(e)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = match self {
            Failure::SlackAPIRequestFailed(e) => format!("Slack API request failed: {:?}", e),
            Failure::SlackAPIResponseError(e) => format!("Slack API returned error: {}", e),
            Failure::SlackAPIResponseMissingError => "Slack API failed to return error.".into(),
            Failure::SlackUnknownChannel(c) => format!("Unknown Slack channel: {}", c),
        };

        write!(f, "{}", x)
    }
}
