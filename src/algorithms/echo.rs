use async_trait::async_trait;
use framework::{self, community::PeerId, transport::Transport, Algorithm};
use serde::{Deserialize, Serialize};

#[framework::message]
struct Message {
    sender: String,
    message: String,
}

#[framework::setup]
pub struct Echo<T: Transport> {
    test: u32,
    #[framework::config(default = false)]
    start_node: bool,
}

#[async_trait]
impl<T: Transport> Algorithm<T> for Echo<T> {
    async fn on_start(&self) {
        let peer = self.peers.values().next().unwrap();

        match peer
            .message(&Message {
                sender: "Test".to_string(),
                message: "Hello, world!".to_string(),
            })
            .await
        {
            Ok(_) => println!("Message sent"),
            Err(e) => println!("Error sending message: {}", e),
        }
    }

    async fn on_exit(&self) {
        println!("Echo algorithm exiting");
    }
}

#[framework::handlers]
impl<T: Transport> Echo<T> {
    async fn message(&self, src: PeerId, msg: &Message) {
        println!("Received message from {}: {}", src.to_string(), msg.message);
    }
}
