use std::time::Duration;

use async_trait::async_trait;
use framework::{self, community::PeerId, Algorithm, SelfTerminating};
use log::{error, info};

#[framework::message]
struct Message {
    sender: String,
    message: String,
}

#[framework::state]
pub struct Echo {
    #[framework::config(default = false)]
    start_node: bool,
}

#[async_trait]
impl Algorithm for Echo {
    async fn on_start(&self) {
        info!("Echo algorithm starting");
        if self.start_node {
            for peer in self.peers.values() {
                match peer
                    .message(&Message {
                        sender: "Test".to_string(),
                        message: "Hello, world!".to_string(),
                    })
                    .await
                {
                    Ok(Some(message)) => info!("Message echoed: {}", message),
                    Ok(None) => error!("Message not echoed"),
                    Err(e) => error!("Error echoing message: {}", e),
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.terminate().await;
    }
    async fn on_exit(&self) {
        info!("Echo algorithm exiting");
    }
}

#[framework::handlers]
impl Echo {
    async fn message(&self, src: PeerId, msg: &Message) -> Option<String> {
        info!("Received message from {}: {}", src.to_string(), msg.message);
        Some(msg.message.clone())
    }
}
