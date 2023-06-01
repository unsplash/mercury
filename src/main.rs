use slack::channel::ChannelName;
use slack::message::post_message;

pub mod error;
pub mod slack;

#[tokio::main]
async fn main() {
    let res = post_message(ChannelName("playground".into()), &"Ciao! - Mercury".into()).await;

    match res {
        Ok(_) => println!("ok"),
        Err(e) => println!("not ok: {}", e),
    }
}

#[test]
fn example() {
    assert_eq!(2i32 + 3, 5);
}
