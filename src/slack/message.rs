//! Send structured messages to any given Slack channel.

use super::{api::*, block::*, channel::*, mention::*, SlackAccessToken, SlackError};
use serde::{Deserialize, Serialize};
use url::Url;

/// A structured message which does not permit custom formatting.
///
/// The definition is intentionally a little generalised to reduce coupling to
/// Slack and avoid any issues with escaping with the fewest compromises.
#[derive(Deserialize)]
pub struct Message {
    pub channel: ChannelName,
    pub title: String,
    pub desc: String,
    pub link: Option<Url>,
    pub cc: Option<Mention>,
    pub avatar: Option<Url>,
}

/// <https://api.slack.com/methods/chat.postMessage#args>
#[derive(Serialize)]
struct MessageRequest<'a> {
    channel: &'a ChannelId,
    username: String,
    blocks: Vec<Block>,
    icon_url: Option<Url>,
    // Used for notifications in the presence of `blocks`.
    text: String,
}

/// <https://api.slack.com/methods/chat.postMessage#examples>
#[derive(Deserialize)]
struct MessageResponse {
    #[allow(dead_code)]
    #[serde(deserialize_with = "crate::de::only_true")]
    ok: bool,
}

impl SlackClient {
    /// Post a message in a channel, joining it if necessary.
    pub async fn post_message(
        &mut self,
        msg: &Message,
        token: &SlackAccessToken,
    ) -> Result<(), SlackError> {
        let channel_id = self.get_channel_id(&msg.channel, token).await?;

        let res = self.try_post_message(&channel_id, msg, token).await;

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                // If we've failed to post the message because we're not in the
                // channel, try joining the channel and posting the message again.
                if is_not_in_channel(&e) {
                    self.join_channel(&channel_id, token).await?;
                    self.try_post_message(&channel_id, msg, token).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Try to post a message assuming we've already joined the channel.
    async fn try_post_message(
        &self,
        channel_id: &ChannelId,
        msg: &Message,
        token: &SlackAccessToken,
    ) -> Result<(), SlackError> {
        let res: APIResult<MessageResponse> = self
            .post("/chat.postMessage", token)
            .json(&MessageRequest {
                channel: channel_id,
                username: msg.title.to_owned(),
                blocks: build_blocks(msg),
                icon_url: msg.avatar.to_owned(),
                text: build_notif_text(msg),
            })
            .send()
            .await?
            .json()
            .await?;

        match res {
            APIResult::Ok(_) => Ok(()),
            APIResult::Err(res) => Err(SlackError::APIResponseError(res.error)),
        }
    }
}

/// Parse Slack's API response error to determine if the issue is that we need
/// to join the channel.
fn is_not_in_channel(res: &SlackError) -> bool {
    match res {
        SlackError::APIResponseError(e) => e == "not_in_channel",
        _ => false,
    }
}

/// Put together the blocks, mapping [Message] to its format on Slack's end,
/// including formatting.
fn build_blocks(msg: &Message) -> Vec<Block> {
    let mut xs = Vec::with_capacity(3);

    xs.push(TextObject::Plaintext(msg.desc.to_owned()));

    if let Some(link) = &msg.link {
        // We shouldn't be able to both parse and print something as a `Url` and
        // also achieve mrkdwn formatting.
        xs.push(TextObject::Mrkdwn(fmt_link(link)));
    }

    if let Some(cc) = &msg.cc {
        xs.push(TextObject::Mrkdwn(fmt_mention(cc)));
    }

    vec![Block::Context(xs)]
}

fn build_notif_text(msg: &Message) -> String {
    format!("{}: {}", msg.title, msg.desc)
}

/// Format a [Mention] to the syntax Slack expects, and stylise it.
fn fmt_mention(m: &Mention) -> String {
    format!("cc <!subteam^{}>", to_user_group_id(m))
}

/// Prettify a URL, reducing verbosity.
///
/// ```
/// let url = "https://unsplash.com/it?set_locale=it-IT";
/// assert_eq!(
///     fmt_link(&Url::parse(url).unwrap()),
///     format!("<{}|unsplash.com/it>", url)
/// );
/// ```
/// Format a [Url] to Slack mrkdwn syntax, expressed as an emoji.
fn fmt_link(u: &Url) -> String {
    format!("<{}|{}>", u, "↗")
}
