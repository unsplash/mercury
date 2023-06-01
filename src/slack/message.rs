use super::{api::*, channel::*};
use crate::error::Failure;
use serde::{Deserialize, Serialize};

/// An opinionated, structured message.
pub struct Message {
    pub channel: ChannelName,
    pub title: String,
    pub desc: String,
}

/// https://api.slack.com/methods/chat.postMessage#args
#[derive(Serialize)]
struct MessageRequest<'a> {
    channel: &'a ChannelId,
    text: String,
}

/// https://api.slack.com/methods/chat.postMessage#examples
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
            text: build_text(&msg),
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

fn build_text(msg: &Message) -> String {
    format!("*{}*\n{}", msg.title, msg.desc)
}
