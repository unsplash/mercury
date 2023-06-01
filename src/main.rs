use slack::channel::ChannelName;
use slack::message::{post_message, Message};

pub mod error;
pub mod slack;

#[tokio::main]
async fn main() {
    let msg = Message {
        channel: ChannelName("playground".into()),
        title: "A title".into(),
        desc: "And a description.".into(),
    };

    let res = post_message(&msg).await;

    match res {
        Ok(_) => println!("ok"),
        Err(e) => println!("not ok: {}", e),
    }
}

#[test]
fn example() {
    assert_eq!(2i32 + 3, 5);
}
