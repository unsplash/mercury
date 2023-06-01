use super::{api::*, channel::*};
use crate::error::Failure;
use serde::{Deserialize, Serialize};

/// https://api.slack.com/methods/chat.postMessage#args
#[derive(Serialize)]
struct MessageRequest<'a> {
    channel: &'a ChannelId,
    text: &'a String,
}

/// https://api.slack.com/methods/chat.postMessage#examples
#[derive(Deserialize)]
pub struct MessageResponse {
    ok: bool,
    error: Option<String>,
}

/// Try to post a message in a channel, joining it if necessary.
pub async fn post_message(channel_name: ChannelName, msg: &String) -> Result<(), Failure> {
    let channel_id = get_channel_id(channel_name).await?;

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
async fn try_post_message(channel: &ChannelId, msg: &String) -> Result<(), Failure> {
    let res: MessageResponse = post("/chat.postMessage")
        .json(&MessageRequest { channel, text: msg })
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
