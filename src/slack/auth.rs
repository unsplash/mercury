use dotenvy_macro::dotenv;

pub const TOKEN: &'static str = dotenv!("SLACK_TOKEN");
