//! Receive webhooks for rollbacks and environment variable changes from Heroku.
//!
//! Requests are validated with `$HEROKU_SECRET`. Any arbitrary text can act as
//! a secret. If `$HEROKU_SECRET` changes then any previously defined webhooks
//! will be invalid and their requests will fail.
//!
//! Webhooks must be created externally, supplying Mercury's
//! `/api/v1/heroku/hook` endpoint as the target URL. The `platform` query param
//! determines where messages are sent, along with platform-specific metadata.
//!
//! Currently the only supported platform is [Slack][super::slack] which takes
//! an additional `channel` query param, for example
//! `/api/v1/heroku/hook?platform=slack&channel=playground`. The message
//! structure is fixed.

pub mod auth;
mod platforms;
pub mod router;
