use slack::channel::ChannelName;
use slack::message::{post_message, Message};
use url::Url;

pub mod error;
pub mod slack;

#[tokio::main]
async fn main() {
    let msg = Message {
        channel: ChannelName("playground".into()),
        title: "A title".into(),
        desc: "*Unformatted text.* 😄".into(),
        links: vec![
            Url::parse("https://unsplash.com").unwrap(),
            Url::parse(
                "https://github.com/unsplash/unsplash-web/issues/8083#issuecomment-1154263409",
            )
            .unwrap(),
        ],
    };

    let res = post_message(&msg).await;

    match res {
        Ok(_) => println!("ok"),
        Err(e) => println!("not ok: {}", e),
    }
}
