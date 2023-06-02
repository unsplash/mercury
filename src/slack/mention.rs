use serde::Deserialize;

/// We could potentially reverse engineer user group IDs from friendly names
/// like we do for channels as per:
///   <https://api.slack.com/reference/surfaces/formatting#mentioning-groups>
///
/// However that'd imply that all consumers have to keep track of group names
/// and couldn't supply a shorthand to our API. Additionally, exact names aside,
/// groups are unlikely to change very often. Thus we'll hardcode some supported
/// groups instead.
#[derive(Deserialize)]
pub enum Mention {
    #[serde(rename = "web")]
    WebTeam,
    #[serde(rename = "api")]
    APITeam,
}

pub fn to_user_group_id(m: &Mention) -> &'static str {
    match m {
        Mention::WebTeam => "SAWPVDSUW",
        Mention::APITeam => "SAVLBV4J0",
    }
}
