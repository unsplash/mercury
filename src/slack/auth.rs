use dotenvy_macro::dotenv;

pub const TOKEN: &str = dotenv!("SLACK_TOKEN");
