use async_trait::async_trait;
use framework::{self, community::PeerId, transport::Transport, Algorithm, SelfTerminating};
use log::{error, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[framework::message]
struct ElectionMessage {
    elector: u32,
}

#[framework::message]
struct TerminationMessage {
    terminate: bool,
}

#[framework::state]
pub struct RingElection<T: Transport> {
    #[framework::config]
    node_id: String,
}

#[async_trait]
impl<T: Transport> Algorithm<T> for RingElection<T> {
    async fn on_start(&self) {
        let delay = rand::thread_rng().gen_range(1.0..3.0);
        sleep(Duration::from_secs_f64(delay)).await;

        if let Some((peer_id, peer)) = self.peers.iter().next() {
            info!(
                "[Node {}] Starting by selecting a node: {}",
                self.node_id, peer_id
            );

            let node_id_num: u32 = self.node_id.parse().unwrap_or(0);
            match peer
                .election(&ElectionMessage {
                    elector: node_id_num,
                })
                .await
            {
                Ok(_) => {}
                Err(e) => error!(
                    "[Node {}] Error sending election message: {}",
                    self.node_id, e
                ),
            }
        }
    }
}

#[framework::handlers]
impl<T: Transport> RingElection<T> {
    async fn election(&self, src: PeerId, msg: &ElectionMessage) {
        let next_peer = self
            .peers
            .iter()
            .find(|(peer_id, _)| **peer_id != src)
            .map(|(peer_id, peer)| (peer_id.clone(), peer));

        if let Some((_next_node_id, next_peer)) = next_peer {
            info!(
                "[Node {}] Got a message with elector id: {}",
                self.node_id, msg.elector
            );

            let received_id = msg.elector;
            let node_id_num: u32 = self.node_id.parse().unwrap_or(0);

            if received_id == node_id_num {
                // We are elected
                info!("[Node {}] we are elected!", self.node_id);
                info!(
                    "[Node {}] Sending message to terminate the algorithm!",
                    self.node_id
                );

                match next_peer
                    .termination(&TerminationMessage { terminate: true })
                    .await
                {
                    Ok(_) => {}
                    Err(e) => error!(
                        "[Node {}] Error sending termination message: {}",
                        self.node_id, e
                    ),
                }
            } else if received_id < node_id_num {
                // Send self.node_id along
                match next_peer
                    .election(&ElectionMessage {
                        elector: node_id_num,
                    })
                    .await
                {
                    Ok(_) => {}
                    Err(e) => error!(
                        "[Node {}] Error sending election message: {}",
                        self.node_id, e
                    ),
                }
            } else {
                // received_id > node_id_num
                // Send received_id along
                match next_peer
                    .election(&ElectionMessage {
                        elector: received_id,
                    })
                    .await
                {
                    Ok(_) => {}
                    Err(e) => error!(
                        "[Node {}] Error sending election message: {}",
                        self.node_id, e
                    ),
                }
            }
        }
    }

    async fn termination(&self, src: PeerId, _msg: &TerminationMessage) {
        // Get the next peer (the one that is NOT the source)
        let next_peer = self
            .peers
            .iter()
            .find(|(peer_id, _)| **peer_id != src)
            .map(|(_, peer)| peer);

        if let Some(next_peer) = next_peer {
            match next_peer
                .termination(&TerminationMessage { terminate: true })
                .await
            {
                Ok(_) => {}
                Err(e) => error!(
                    "[Node {}] Error sending termination message: {}",
                    self.node_id, e
                ),
            }
        }

        self.terminate().await;
    }
}
