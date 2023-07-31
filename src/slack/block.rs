//! Slack's block API [^blocks-api] is its most modern, and allows us to mix
//! rich formatting with foreign plaintext.
//!
//! It has some limitations however:
//! - It doesn't allow one to mix rich text and plaintext in a single "section".
//!   - `rich_text` et al are unsupported as inputs to the API.
//! - Accessory buttons insist upon on onward webhook URL [^button-webhook].
//! - The messages tend towards being very large.
//!
//! Considering the alternative, "attachments", are deprecated, we'll make do
//! with some basic blocks, utilising context blocks for smaller copy.
//!
//! [^blocks-api]: <https://api.slack.com/reference/block-kit/blocks>
//!
//! [^button-webhook]: <https://stackoverflow.com/questions/64107123/can-you-use-slack-buttons-non-interactively>

use serde::ser::SerializeStruct;
use serde::{ser, Serialize};

/// A simplified representation of Slack's "blocks", supporting only the bare
/// minimum we need to achieve our desired outcome.
pub enum Block {
    /// Ordinary, standalone copy.
    Section(TextObject),
    /// Small copy. The items are rendered compactly together.
    Context(Vec<TextObject>),
}

impl ser::Serialize for Block {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Block", 2)?;

        match self {
            Block::Section(x) => {
                state.serialize_field("type", "section")?;
                state.serialize_field("text", x)?;
            }
            Block::Context(xs) => {
                state.serialize_field("type", "context")?;
                state.serialize_field("elements", xs)?;
            }
        };

        state.end()
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "text")]
pub enum TextObject {
    /// Plaintext, safe for foreign input.
    #[serde(rename = "plain_text")]
    Plaintext(String),
    /// Slack's take on markdown, unsafe for foreign input.
    #[serde(rename = "mrkdwn")]
    Mrkdwn(String),
}
