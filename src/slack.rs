//! Supports posting a structured message to any Slack channel. The
//! [message format][message::Message] is intentionally a little generalised.
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
//! ```
//!
//! The permission scopes serve the following purposes:
//!
//! - `channels:read`: Map channel names to channel IDs.
//! - `channels:join`: Join channels automatically.
//! - `chat:write`: Send messages to channels.
//!
//! `channels:join` is optional if you manually add the bot to the channels
//! you'd like to post to.

mod api;
pub mod auth;
mod block;
pub mod channel;
pub mod error;
pub mod mention;
pub mod message;
