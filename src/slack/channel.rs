use cached::proc_macro::cached;
use serde_with::{serde_as, NoneAsEmptyString};
use std::{collections::HashMap, fmt};

use super::api::*;
use crate::error::Failure;
use serde::{Deserialize, Serialize};

/// Channel names as are visible in the Slack UI, absent the leading hash.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelName(pub String);

/// Format without the surrounding newtype wrapper i.e. `foo` instead of
/// `ChannelName("foo")`.
impl fmt::Display for ChannelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Because channel names can change, channels are generally referred to by
/// their underlying ID. This can be found in the UI by copying a link to the
/// channel.
#[derive(Clone, Serialize, Deserialize)]
pub struct ChannelId(pub String);

/// Slack's API expects channel IDs, however we want consumers to be able to
/// supply channel names without worrying about that detail.
type ChannelMap = HashMap<ChannelName, ChannelId>;

#[derive(Deserialize)]
struct ChannelMeta {
    id: ChannelId,
    name: ChannelName,
}

/// <https://api.slack.com/methods/conversations.join#args>
#[derive(Serialize)]
struct JoinRequest<'a> {
    channel: &'a ChannelId,
}

/// <https://api.slack.com/methods/conversations.join#examples>
#[derive(Deserialize)]
struct JoinResponse {
    ok: bool,
    error: Option<String>,
}

/// We just join channels before we can message in them.
pub async fn join_channel(channel: &ChannelId) -> Result<(), Failure> {
    let res: JoinResponse = post("/conversations.join")
        .json(&JoinRequest { channel })
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

pub async fn get_channel_id(channel_name: &ChannelName) -> Result<ChannelId, Failure> {
    let map = get_channel_map().await?;

    map.get(channel_name)
        .ok_or(Failure::SlackUnknownChannel(channel_name.clone()))
        .cloned()
}

/// <https://api.slack.com/methods/conversations.list#args>
#[derive(Serialize)]
struct ListRequest {
    /// Maximum supported is 1000, but a limit of 200 is "recommended".
    limit: u16,
    /// Doesn't affect `limit`.
    exclude_archived: bool,
    cursor: Option<String>,
}

/// <https://api.slack.com/methods/conversations.list#examples>
#[derive(Deserialize)]
struct ListResponse {
    ok: bool,
    error: Option<String>,
    channels: Vec<ChannelMeta>,
    response_metadata: PaginationMeta,
}

#[serde_as]
#[derive(Deserialize)]
struct PaginationMeta {
    #[serde_as(as = "NoneAsEmptyString")]
    next_cursor: Option<String>,
}

/// Get a map from channel names to channel IDs. The first successful result of
/// this function is cached, meaning that there's a risk of the map becoming
/// stale should channels be renamed.
#[cached(result = true, sync_writes = true)]
async fn get_channel_map() -> Result<ChannelMap, Failure> {
    let mut channels: Vec<ChannelMeta> = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut res: ListResponse = get("/conversations.list")
            .query(&ListRequest {
                limit: 200,
                exclude_archived: true,
                cursor,
            })
            .send()
            .await?
            .json()
            .await?;

        if res.ok {
            channels.append(&mut res.channels);

            cursor = res.response_metadata.next_cursor;
            if cursor.is_some() {
                continue;
            }

            let map: ChannelMap = channels
                .into_iter()
                .map(|meta| (meta.name, meta.id))
                .collect();

            break Ok(map);
        } else {
            break Err(decode_error(res.error));
        }
    }
}
