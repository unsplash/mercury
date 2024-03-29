//! Post a structured [Message] to any Slack channel.
//!
//! To get started and obtain an access token, create a Slack app with the
//! following app manifest:
//!
//! ```yaml
//! display_information:
//!   name: Mercury
//!   description: The guide of souls to the underworld.
//!   background_color: "#d40b01"
//! features:
//!   bot_user:
//!     display_name: Mercury
//! oauth_config:
//!   scopes:
//!     bot:
//!       - channels:read
//!       - channels:join
//!       - chat:write
//!       - chat:write.customize
//! ```
//!
//! The permission scopes serve the following purposes:
//!
//! - `channels:read`: Map channel names to channel IDs.
//! - `channels:join`: Join channels automatically.
//! - `chat:write`: Send messages to channels.
//! - `chat:write.customize`: Terser messages utilising the username, and custom
//! avatars.
//!
//! `channels:join` is optional if you manually add the bot to the channels
//! you'd like to post to.

pub mod api;
pub mod auth;
mod block;
pub mod channel;
pub mod error;
mod mention;
pub mod message;
pub mod router;

pub use api::SlackClient;
pub use auth::SlackAccessToken;
pub use error::SlackError;
pub use message::Message;
