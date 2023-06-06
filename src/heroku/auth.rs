//! Helpers around Heroku's use of a shared secret to authenticate webhook
//! requests.

/// A newtype wrapper around the Heroku secret.
pub struct HerokuSecret(pub String);
