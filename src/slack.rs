//! Supports posting a structured message to any Slack channel.
//!
//! Whilst Mercury currently only supports this communication mechanism, the
//! structure is intentionally a little generalised.
//!
//! See [message::Message].

mod api;
pub mod auth;
mod block;
pub mod channel;
pub mod error;
pub mod mention;
pub mod message;
