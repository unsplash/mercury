//! Messaging platforms for successful Heroku webhook requests.

use self::slack::SlackPlatform;
use serde::Deserialize;

pub(super) mod slack;

/// Supported onward platforms.
#[derive(Deserialize)]
#[serde(tag = "platform")]
pub enum Platform {
    /// Post a fixed message to the specified Slack channel.
    #[serde(rename = "slack")]
    Slack(SlackPlatform),
}
