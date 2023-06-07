//! Helpers around Slack's use of OAuth Bearer Authentication.

/// A newtype wrapper around Slack access tokens.
#[derive(Clone)]
pub struct SlackAccessToken(pub String);

/// Convert a Slack access token to a `Bearer` `Authorization` header value.
///
/// ```
/// let token = SlackAccessToken("xoxb-foo".into());
/// assert_eq!(to_auth_header_val(&token), "Bearer xoxb-foo");
/// ```
pub fn to_auth_header_val(t: &SlackAccessToken) -> String {
    format!("Bearer {}", t.0)
}
