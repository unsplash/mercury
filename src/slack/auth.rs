//! Helpers around Slack's use of OAuth Bearer Authentication.

/// A newtype wrapper around Slack access tokens.
// The derived trait implementations are for [super::channel::get_channel_map]'s
// use of [cached::cached]. The alternative at that call site would be:
//   <https://github.com/jaemk/cached/issues/135>
#[derive(PartialEq, Eq, Hash, Clone)]
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
