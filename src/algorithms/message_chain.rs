use std::{
    collections::HashMap,
    fmt::{self, Display},
    sync::Mutex,
};

use async_trait::async_trait;
use common_macros::hash_map;
use distbench::{self, community::PeerId, signing::Signed, Algorithm, SelfTerminating};
use log::info;

/// A message that contains a payload and can be signed
#[distbench::message]
struct ChainMessage {
    hop_count: u32,
    original_value: String,
    /// Just to demonstrate: the node that created this hop
    node_name: String,
}

impl Display for ChainMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Hop #{} by {} (value: '{}')",
            self.hop_count, self.node_name, self.original_value
        )
    }
}

/// A container message that holds a vector of signed messages
#[distbench::message]
struct ChainBundle {
    /// Vector of signed messages collected along the chain
    chain: Vec<Signed<ChainMessage>>,
}

#[distbench::state]
pub struct MessageChain {
    /// If true, this node initiates the chain
    #[distbench::config(default = false)]
    is_initiator: bool,

    /// The initial value to send
    #[distbench::config(default = "Chain start!".to_string())]
    initial_value: String,

    /// Maximum number of hops before terminating
    #[distbench::config(default = 5)]
    max_hops: u32,

    /// Collected chains received
    received_chains: Mutex<Vec<Vec<PeerId>>>,
}

#[async_trait]
impl Algorithm for MessageChain {
    async fn on_start(&self) {
        info!(
            "[{}] MessageChain starting (N={} nodes)",
            self.id(),
            self.N()
        );

        if self.is_initiator {
            info!(
                "[{}] I am the initiator, starting chain with value: '{}'",
                self.id(),
                self.initial_value
            );

            // Create initial signed message
            let msg = self.sign(ChainMessage {
                hop_count: 0,
                original_value: self.initial_value.clone(),
                node_name: self.id().to_string(),
            });

            // When we log this Signed<ChainMessage>, it will use its Display impl
            // which shows: "Hop #0 by node1 (value: '...') (signed by node1)"
            info!("[{}] Created initial message: {}", self.id(), msg);

            // Send to all peers with just this one message in the chain
            for (peer_id, peer) in self.peers() {
                if let Err(e) = peer
                    .receive_chain(&ChainBundle {
                        chain: vec![msg.clone()],
                    })
                    .await
                {
                    info!("[{}] Error sending to {}: {}", self.id(), peer_id, e);
                }
            }
        }
    }

    async fn on_exit(&self) {
        info!("[{}] MessageChain exiting", self.id());
    }

    async fn report(&self) -> Option<HashMap<impl Display, impl Display>> {
        let chains = self.received_chains.lock().unwrap();
        Some(hash_map! {
            "chains_received" => chains.len().to_string(),
            "is_initiator" => self.is_initiator.to_string(),
        })
    }
}

#[distbench::handlers]
impl MessageChain {
    async fn receive_chain(&self, src: PeerId, bundle: &ChainBundle) {
        if bundle.chain.is_empty() {
            info!("[{}] Received empty chain from {}", self.id(), src);
            return;
        }

        info!(
            "[{}] Received chain from {} with {} messages",
            self.id(),
            src,
            bundle.chain.len()
        );

        // Demonstrate: Display implementation shows signatures
        // Each Signed<ChainMessage> displays as: "Hop #N by nodeX (value: '...') (signed by nodeX)"
        info!("[{}] Chain messages:", self.id());
        for (i, signed_msg) in bundle.chain.iter().enumerate() {
            info!("[{}]   [{}] {}", self.id(), i, signed_msg);
        }

        // Get info from messages
        let first_msg = bundle.chain.first().unwrap();
        let last_msg = bundle.chain.last().unwrap();

        info!("[{}] First message: {}", self.id(), first_msg);
        info!("[{}] Last message: {}", self.id(), last_msg);

        // Store node names from the chain
        {
            let mut chains = self.received_chains.lock().unwrap();
            let node_path: Vec<PeerId> = bundle
                .chain
                .iter()
                .map(|msg| PeerId::new(msg.node_name.clone()))
                .collect();
            chains.push(node_path);
        }

        // If we haven't reached max hops, extend the chain and forward
        if last_msg.hop_count < self.max_hops {
            info!(
                "[{}] Extending chain (current hops: {}, max: {})",
                self.id(),
                last_msg.hop_count,
                self.max_hops
            );

            // Create our signed message
            let our_msg = self.sign(ChainMessage {
                hop_count: last_msg.hop_count + 1,
                original_value: first_msg.original_value.clone(),
                node_name: self.id().to_string(),
            });

            // Create new chain with all previous messages plus ours
            let mut new_chain = bundle.chain.clone();
            new_chain.push(our_msg);

            info!(
                "[{}] Forwarding extended chain ({} total messages) to {} peers",
                self.id(),
                new_chain.len(),
                self.peers().count()
            );

            // Forward to all peers
            for (peer_id, peer) in self.peers() {
                // Don't send back to the peer we received from
                if *peer_id == src {
                    continue;
                }

                if let Err(e) = peer
                    .receive_chain(&ChainBundle {
                        chain: new_chain.clone(),
                    })
                    .await
                {
                    info!("[{}] Error forwarding to {}: {}", self.id(), peer_id, e);
                }
            }
        } else {
            info!(
                "[{}] Chain reached max hops ({}), terminating",
                self.id(),
                self.max_hops
            );
            self.terminate().await;
        }
    }
}
