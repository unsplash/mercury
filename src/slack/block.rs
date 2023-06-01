use serde::ser::SerializeStruct;
use serde::{ser, Serialize};

/// Slack's block API is its most modern, and allows us to mix rich formatting
/// with foreign plaintext. This is our limited subset thereof.
///
/// <https://api.slack.com/reference/block-kit/blocks>
pub enum Block {
    Header(String),
    Plaintext(String),
    /// "mrkdown" is Slack's alternative to Markdown. It's the same syntax as
    /// we use in the app.
    ///
    /// <https://api.slack.com/reference/surfaces/formatting#basics>
    Mrkdwn(String),
}

// This won't scale to other block types but for now is simpler than a more
// custom serialisation implementation.
#[derive(Serialize)]
struct RawTextBlock<'a> {
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
            Block::Header(x) => {
                state.serialize_field("type", "header")?;

                let inner = RawTextBlock {
                    typ: "plain_text",
                    text: x,
                };
                state.serialize_field("text", &inner)?;
            }

            Block::Mrkdwn(x) => {
                state.serialize_field("type", "section")?;

                let inner = RawTextBlock {
                    typ: "mrkdwn",
                    text: x,
                };
                state.serialize_field("text", &inner)?;
            }

            Block::Plaintext(x) => {
                state.serialize_field("type", "section")?;

                let inner = RawTextBlock {
                    typ: "plain_text",
                    text: x,
                };
                state.serialize_field("text", &inner)?;
            }
        };

        state.end()
    }
}
