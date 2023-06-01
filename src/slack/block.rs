use serde::ser::SerializeStruct;
use serde::{ser, Serialize};

/// Slack's block API is its most modern, and allows us to mix rich formatting
/// with foreign plaintext. It has some limitations however:
///
/// - It doesn't allow one to mix rich text and plaintext in a single "section".
/// - Accessory buttons insist upon on onward webhook URL.
///   - https://stackoverflow.com/questions/64107123/can-you-use-slack-buttons-non-interactively
/// - The messages tend towards being very large.
///
/// Considering the alternative, "attachments", are deprecated, we'll make do
/// with some basic blocks, utilising context blocks for smaller copy.
///
/// <https://api.slack.com/reference/block-kit/blocks>
pub enum Block {
    Plaintext(String),
    Context(String),
}

// This won't scale to other block types but for now is simpler than a more
// custom serialisation implementation.
#[derive(Serialize)]
struct TextObj<'a> {
    #[serde(rename = "type")]
    typ: &'static str,
    text: &'a String,
}

impl ser::Serialize for Block {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Block", 2)?;

        match self {
            Block::Plaintext(x) => {
                state.serialize_field("type", "section")?;

                let inner = TextObj {
                    typ: "plain_text",
                    text: x,
                };
                state.serialize_field("text", &inner)?;
            }

            Block::Context(x) => {
                state.serialize_field("type", "context")?;

                let inner = TextObj {
                    typ: "mrkdwn",
                    text: x,
                };
                state.serialize_field("elements", &vec![inner])?;
            }
        };

        state.end()
    }
}
