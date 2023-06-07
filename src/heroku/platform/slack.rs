//! Send messages to a specified Slack channel on receipt of a Heroku webhook.

use crate::slack::channel::ChannelName;
use serde::Deserialize;

/// Metadata for the Slack platform which the webhook request must supply.
#[derive(Deserialize)]
pub struct SlackPlatform {
    pub channel: ChannelName,
}
