/// These are for `get_channel_map`'s use of <cached::cached>, unless we'd
/// rather mess around with this:
///   https://github.com/jaemk/cached/issues/135
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct SlackAccessToken(pub String);

pub fn to_auth_header_val(t: &SlackAccessToken) -> String {
    format!("Bearer {}", t.0)
}
