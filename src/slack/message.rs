use super::{api::*, block::*, channel::*};
use crate::error::Failure;
use serde::{Deserialize, Serialize};
use url::Url;

/// An opinionated, structured message.
pub struct Message {
    pub channel: ChannelName,
    pub title: String,
    pub desc: String,
    pub links: Vec<Url>,
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
    ok: bool,
    error: Option<String>,
}

/// Try to post a message in a channel, joining it if necessary.
pub async fn post_message(msg: &Message) -> Result<(), Failure> {
    let channel_id = get_channel_id(&msg.channel).await?;

    let res = try_post_message(&channel_id, &msg).await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            // If we've failed to post the message because we're not in the
            // channel, try joining the channel and posting the message again.
            if is_not_in_channel(&e) {
                join_channel(&channel_id).await?;
                try_post_message(&channel_id, &msg).await
            } else {
                Err(e)
            }
        }
    }
}

/// Try to post a message assuming we've already joined the channel.
async fn try_post_message(channel_id: &ChannelId, msg: &Message) -> Result<(), Failure> {
    let res: MessageResponse = post("/chat.postMessage")
        .json(&MessageRequest {
            channel: channel_id,
            blocks: fmt_msg(&msg),
        })
        .send()
        .await?
        .json()
        .await?;

    if res.ok {
        Ok(())
    } else {
        Err(decode_error(res.error))
    }
}

fn is_not_in_channel(res: &Failure) -> bool {
    match res {
        Failure::SlackAPIResponseError(e) => e == "not_in_channel",
        _ => false,
    }
}

fn fmt_msg(msg: &Message) -> Vec<Block> {
    let mut xs = Vec::with_capacity(2);

    xs.push(Block::Header(msg.title.clone()));
    xs.push(Block::Plaintext(msg.desc.clone()));

    if !msg.links.is_empty() {
        // We shouldn't be able to both parse and print something as a `Url` and
        // also achieve mrkdwn formatting.
        xs.push(Block::Mrkdwn(fmt_links(&msg.links)));
    }

    xs
}

fn fmt_links(links: &Vec<Url>) -> String {
    let mut out = String::new();

    for link in links {
        out.push_str("\n• ");
        out.push_str(&fmt_link(&link));
    }

    out
}

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

#[test]
fn test_fmt_link() {
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
