use std::{collections::HashMap, fmt::Display};

use async_trait::async_trait;
use common_macros::hash_map;
use distbench::{self, Algorithm, FormatError, PeerId};
use log::info;
use tokio::sync::Mutex;

// ============================================================================
// Lower Layer: SimpleBroadcast
// ============================================================================

#[distbench::message]
struct SimpleBroadcastMessage {
    payload: Vec<u8>,
}

#[distbench::state]
pub struct SimpleBroadcast {
    #[distbench::config(default = 0)]
    max_retries: u32,

    messages_broadcasted: Mutex<u64>,
}

/// Interface provided to upper layers to broadcast messages.
#[distbench::interface]
impl SimpleBroadcast {
    /// Broadcast a message to all peers and deliver to parent
    pub async fn broadcast(&self, msg: Vec<u8>) -> Result<(), FormatError> {
        info!(
            "SimpleBroadcast: Broadcasting message to {} peers",
            self.peers().count()
        );

        // Wrap the AlgorithmMessage in our own message type for peer-to-peer communication
        let broadcast_msg = SimpleBroadcastMessage {
            payload: msg.clone(),
        };

        for (peer_id, peer) in self.peers() {
            match peer.simple_broadcast_message(&broadcast_msg).await {
                Ok(_) => {
                    info!("SimpleBroadcast: Sent to peer {}", peer_id);
                }
                Err(e) => {
                    info!("SimpleBroadcast: Error sending to peer {}: {}", peer_id, e);
                }
            }
        }

        *self.messages_broadcasted.lock().await += 1;

        // Deliver to parent layer
        if let Err(e) = self.deliver(self.id().clone(), &msg).await {
            info!("SimpleBroadcast: Failed to deliver to parent: {}", e);
        }

        Ok(())
    }
}

#[async_trait]
impl Algorithm for SimpleBroadcast {
    async fn on_start(&self) {
        info!(
            "SimpleBroadcast starting with max_retries: {}",
            self.max_retries
        );
    }

    async fn on_exit(&self) {
        info!("SimpleBroadcast exiting");
    }

    async fn report(&self) -> Option<HashMap<impl Display, impl Display>> {
        Some(hash_map! {
            "messages_broadcasted" => self.messages_broadcasted.lock().await.to_string(),
            "max_retries" => self.max_retries.to_string(),
        })
    }
}

#[distbench::handlers]
impl SimpleBroadcast {
    async fn simple_broadcast_message(&self, src: PeerId, msg: &SimpleBroadcastMessage) {
        info!(
            "SimpleBroadcast: Received message from {} with {} bytes",
            src,
            msg.payload.len()
        );

        // Extract the inner AlgorithmMessage and deliver to parent layer
        if let Err(e) = self.deliver(src, &msg.payload).await {
            info!("SimpleBroadcast: Failed to deliver to parent: {}", e);
        }
    }
}

// ============================================================================
// Upper Layer: SimpleBroadcastUpper
// ============================================================================

#[distbench::message]
pub struct BroadcastSend {
    pub content: Vec<u8>,
    pub sequence: u32,
}

#[distbench::message]
pub struct BroadcastEcho {
    pub content: Vec<u8>,
    pub original_sender: String,
}

#[distbench::state]
pub struct SimpleBroadcastUpper {
    #[distbench::config(default = false)]
    start_node: bool,

    #[distbench::config(default = vec![])]
    messages: Vec<String>,

    send_received: Mutex<Vec<BroadcastSend>>,
    echo_received: Mutex<Vec<BroadcastEcho>>,

    #[distbench::child]
    broadcast: SimpleBroadcast,
}

#[async_trait]
impl Algorithm for SimpleBroadcastUpper {
    async fn on_start(&self) {
        info!(
            "SimpleBroadcastUpper starting | start_node: {}, messages: {:?}",
            self.start_node, self.messages
        );

        if self.start_node {
            info!("SimpleBroadcastUpper: Initiating broadcast through lower layer");

            // Broadcast BroadcastSend messages
            for (i, message) in self.messages.iter().enumerate() {
                let send_msg = BroadcastSend {
                    content: message.as_bytes().to_vec(),
                    sequence: i as u32,
                };

                if let Err(e) = self.broadcast.broadcast(&send_msg).await {
                    info!("SimpleBroadcastUpper: Error broadcasting send: {}", e);
                }
            }

            info!("SimpleBroadcastUpper: Initiating broadcast through lower layer, echo");

            // Also broadcast a BroadcastEcho message
            let echo_msg = BroadcastEcho {
                content: b"Echo from upper layer".to_vec(),
                original_sender: self.id().to_string(),
            };

            if let Err(e) = self.broadcast.broadcast(&echo_msg).await {
                info!("SimpleBroadcastUpper: Error broadcasting echo: {}", e);
            }
        }
    }

    async fn on_exit(&self) {
        info!("SimpleBroadcastUpper exiting");
    }

    async fn report(&self) -> Option<HashMap<impl Display, impl Display>> {
        Some(hash_map! {
            "send_received" => self.send_received.lock().await.len().to_string(),
            "echo_received" => self.echo_received.lock().await.len().to_string(),
            "start_node" => self.start_node.to_string(),
        })
    }
}

#[distbench::handlers]
impl SimpleBroadcastUpper {}

#[distbench::handlers(from = broadcast)]
impl SimpleBroadcastUpper {
    async fn broadcast_send(&self, src: PeerId, msg: &BroadcastSend) {
        info!(
            "SimpleBroadcastUpper: Received BroadcastSend from {} | sequence: {}, {} bytes",
            src,
            msg.sequence,
            msg.content.len()
        );
        let mut send_received = self.send_received.lock().await;
        send_received.push(msg.clone());
    }

    async fn broadcast_echo(&self, src: PeerId, msg: &BroadcastEcho) {
        info!(
            "SimpleBroadcastUpper: Received BroadcastEcho from {} | original_sender: {}, {} bytes",
            src,
            msg.original_sender,
            msg.content.len()
        );
        let mut echo_received = self.echo_received.lock().await;
        echo_received.push(msg.clone());
    }
}
