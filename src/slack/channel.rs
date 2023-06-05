//! Interact with Slack channels, including the ability to programmatically
//! join them.

use super::{api::*, auth::SlackAccessToken, error::SlackError};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use std::{collections::HashMap, fmt};

/// Channel names as are visible in the Slack UI, with or without the leading
/// hash.
///
/// ```
/// let with =    ChannelName("#playground".into());
/// let without = ChannelName("playground".into());
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelName(pub String);

/// Format without the surrounding newtype wrapper.
///
/// ```
/// let x = ChannelName("fp".into());
/// assert_eq!(format!("{}", x), "fp");
/// ```
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

/// Maps Slack channel names to channel IDs; Slack's API expects channel IDs,
/// however we want consumers to be able to supply channel names without
/// worrying about that detail.
pub type ChannelMap = HashMap<ChannelName, ChannelId>;

/// The metadata we care about per-channel within [ListResponse].
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
    #[allow(dead_code)]
    #[serde(deserialize_with = "crate::de::only_true")]
    ok: bool,
}

impl SlackClient {
    /// We just join channels before we can message in them.
    pub async fn join_channel(
        &self,
        channel: &ChannelId,
        token: &SlackAccessToken,
    ) -> Result<(), SlackError> {
        let res: APIResult<JoinResponse> = self
            .post("/conversations.join", token)
            .json(&JoinRequest { channel })
            .send()
            .await?
            .json()
            .await?;

        match res {
            APIResult::Ok(_) => Ok(()),
            APIResult::Err(res) => Err(SlackError::APIResponseError(res.error)),
        }
    }

    /// Get the channel ID assocatiated with a channel name, enabling onward calls
    /// to Slack's API.
    pub async fn get_channel_id(
        &mut self,
        channel_name: &ChannelName,
        token: &SlackAccessToken,
    ) -> Result<ChannelId, SlackError> {
        let map = self.get_channel_map(token.clone()).await?;

        // Channel names can't contain hashes, so by doing this we can support
        // consumers supplying (or not) a leading hash.
        let normalised_channel_name = ChannelName(channel_name.0.trim_start_matches('#').into());

        map.get(&normalised_channel_name)
            .ok_or(SlackError::UnknownChannel(channel_name.clone()))
            .cloned()
    }
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
    #[allow(dead_code)]
    #[serde(deserialize_with = "crate::de::only_true")]
    ok: bool,
    channels: Vec<ChannelMeta>,
    response_metadata: PaginationMeta,
}

/// The metadata attached to a [ListResponse], enabling pagination.
#[serde_as]
#[derive(Deserialize)]
struct PaginationMeta {
    #[serde_as(as = "NoneAsEmptyString")]
    next_cursor: Option<String>,
}

impl SlackClient {
    /// Get a map from channel names to channel IDs. The first successful result of
    /// this function is cached, meaning that there's a risk of the map becoming
    /// stale should channels be renamed.
    async fn get_channel_map(&mut self, token: SlackAccessToken) -> Result<ChannelMap, SlackError> {
        match &self.channel_map {
            Some(x) => Ok(x.to_owned()),
            None => {
                let mut channels: Vec<ChannelMeta> = Vec::new();
                let mut cursor: Option<String> = None;

                loop {
                    let res: APIResult<ListResponse> = self
                        .get("/conversations.list", &token)
                        .query(&ListRequest {
                            limit: 200,
                            exclude_archived: true,
                            cursor,
                        })
                        .send()
                        .await?
                        .json()
                        .await?;

                    match res {
                        APIResult::Ok(mut res) => {
                            channels.append(&mut res.channels);

                            cursor = res.response_metadata.next_cursor;
                            if cursor.is_some() {
                                continue;
                            }

                            let map: ChannelMap = channels
                                .into_iter()
                                .map(|meta| (meta.name, meta.id))
                                .collect();

                            self.channel_map = Some(map.to_owned());
                            break Ok(map);
                        }
                        APIResult::Err(res) => break Err(SlackError::APIResponseError(res.error)),
                    }
                }
            }
        }
    }
}
