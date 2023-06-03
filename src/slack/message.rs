//! Send structured messages to any given Slack channel.

use super::{api::*, auth::SlackAccessToken, block::*, channel::*, error::SlackError, mention::*};
use serde::{Deserialize, Serialize};
use url::Url;

/// A structured message which does not permit custom formatting.
///
/// The definition is intentionally a little generalised to reduce coupling to
/// Slack and avoid any issues with escaping with the fewest compromises.
///
// It would be nontrivially difficult to support multiple inputs (vectors) with
// `application/x-www-form-urlencoded` bodies:
//   <https://github.com/nox/serde_urlencoded/issues/52>
#[derive(Deserialize)]
pub struct Message {
    pub channel: ChannelName,
    pub title: String,
    pub desc: String,
    pub link: Option<Url>,
    pub cc: Option<Mention>,
}

/// <https://api.slack.com/methods/chat.postMessage#args>
#[derive(Serialize)]
struct MessageRequest<'a> {
    channel: &'a ChannelId,
    blocks: Vec<Block>,
}

/// <https://api.slack.com/methods/chat.postMessage#examples>
#[derive(Deserialize)]
struct MessageResponse {
    #[allow(dead_code)]
    #[serde(deserialize_with = "crate::de::only_true")]
    ok: bool,
}

/// Post a message in a channel, joining it if necessary.
pub async fn post_message(msg: &Message, token: &SlackAccessToken) -> Result<(), SlackError> {
    let channel_id = get_channel_id(&msg.channel, token).await?;

    let res = try_post_message(&channel_id, msg, token).await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            // If we've failed to post the message because we're not in the
            // channel, try joining the channel and posting the message again.
            if is_not_in_channel(&e) {
                join_channel(&channel_id, token).await?;
                try_post_message(&channel_id, msg, token).await
            } else {
                Err(e)
            }
        }
    }
}

/// Try to post a message assuming we've already joined the channel.
async fn try_post_message(
    channel_id: &ChannelId,
    msg: &Message,
    token: &SlackAccessToken,
) -> Result<(), SlackError> {
    let res: APIResult<MessageResponse> = post("/chat.postMessage", token)
        .json(&MessageRequest {
            channel: channel_id,
            blocks: build_blocks(msg),
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

    xs.push(Block::Plaintext(format!("{}: {}", msg.title, msg.desc)));

    if let Some(cc) = &msg.cc {
        xs.push(Block::Mrkdown(fmt_mention(cc)));
    }

    if let Some(link) = &msg.link {
        // We shouldn't be able to both parse and print something as a `Url` and
        // also achieve mrkdwn formatting.
        xs.push(Block::Context(fmt_link(link)));
    }

    xs
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
fn fmt_link(u: &Url) -> String {
    let href = u.to_string();

    // Formats most links to a prettier format, falling back to the href.
    if let Some(host) = u.host_str() {
        let host_sans_www = host.trim_start_matches("www.");

        let path = u.path();
        let path_or_empty = if path == "/" { "" } else { path };
        format!("<{}|{}{}>", href, host_sans_www, path_or_empty)
    } else {
        href
    }
}

#[cfg(test)]
mod tests {
    use super::Url;

    #[test]
    fn test_fmt_link() {
        use super::fmt_link;

        let pretty_raw = "https://images.unsplash.com/path/to/photo.jpg?size=large";
        let pretty = Url::parse(pretty_raw).unwrap();
        assert_eq!(
            fmt_link(&pretty),
            format!("<{}|images.unsplash.com/path/to/photo.jpg>", pretty_raw)
        );

        let pretty_www_raw = "https://www.unsplash.com/path/to/photo.jpg?size=large";
        let pretty_www = Url::parse(pretty_www_raw).unwrap();
        assert_eq!(
            fmt_link(&pretty_www),
            format!("<{}|unsplash.com/path/to/photo.jpg>", pretty_www_raw)
        );

        let pretty_no_path_raw = "https://unsplash.com/";
        let pretty_no_path = Url::parse(pretty_no_path_raw).unwrap();
        assert_eq!(
            fmt_link(&pretty_no_path),
            format!("<{}|unsplash.com>", pretty_no_path_raw)
        );

        let ugly_raw = "data:text/plain,Hello?World#";
        let ugly = Url::parse(ugly_raw).unwrap();
        assert_eq!(fmt_link(&ugly), ugly_raw);
    }
}
