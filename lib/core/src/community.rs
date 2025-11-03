//! Community management for distributed systems.
//!
//! This module defines the [`Community`] type which represents a collection of
//! peers that communicate with each other in a distributed system.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{status::NodeStatus, transport};

/// A public key for cryptographic operations.
pub type PublicKey = Vec<u8>;

/// Unique identifier for a peer in the distributed system.
///
/// `PeerId` is used to identify nodes throughout the system. It wraps a string
/// identifier and implements the necessary traits for use as a key in collections
/// and as a transport address.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct PeerId(String);

impl PeerId {
    /// Creates a new peer ID from a string.
    ///
    /// # Arguments
    ///
    /// * `id` - The string identifier for this peer
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Converts this peer ID to a string.
    ///
    /// # Returns
    ///
    /// A clone of the underlying string identifier.
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

/// A community of peers that communicate with each other.
///
/// The `Community` manages:
/// - The set of neighbor peers for a given node
/// - Connection managers for communicating with each peer
/// - Address-to-PeerId mappings
/// - Status tracking for all peers
/// - Public keys for peers (if applicable)
///
/// # Type Parameters
///
/// * `T` - The transport layer implementation
pub struct Community<T: transport::Transport> {
    neighbours: HashSet<PeerId>,
    connections: HashMap<PeerId, Arc<dyn transport::ConnectionManager<T>>>,
    peers: HashMap<T::Address, PeerId>,
    transport: Arc<T>,
    peer_status: RwLock<HashMap<PeerId, NodeStatus>>,
    keys: RwLock<HashMap<PeerId, PublicKey>>,
}

impl<T: transport::Transport + 'static> Community<T> {
    /// Creates a new community.
    ///
    /// # Arguments
    ///
    /// * `neighbours` - The set of peer IDs that are considered neighbors
    /// * `peer_addresses` - A mapping from peer IDs to their network addresses
    /// * `transport` - The transport layer to use for communication
    ///
    /// # Returns
    ///
    /// A new `Community` instance configured with the provided peers and transport.
    pub fn new(
        neighbours: HashSet<PeerId>,
        peer_addresses: HashMap<PeerId, T::Address>,
        transport: Arc<T>,
    ) -> Self {
        let connections: HashMap<PeerId, Arc<dyn transport::ConnectionManager<T>>> = peer_addresses
            .iter()
            .map(|(id, addr)| {
                let conn_manager: Arc<dyn transport::ConnectionManager<T>> = Arc::new(
                    transport::ThinConnectionManager::new(transport.clone(), addr.clone()),
                );
                (id.clone(), conn_manager)
            })
            .collect();

        let peers = peer_addresses
            .iter()
            .map(|(id, addr)| (addr.clone(), id.clone()))
            .collect();

        Self {
            neighbours,
            connections,
            peers,
            transport,
            keys: Default::default(),
            peer_status: RwLock::new(HashMap::new()),
        }
    }

    /// Sets the status of a peer.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the peer whose status to update
    /// * `status` - The new status for the peer
    pub async fn set_status(&self, id: PeerId, status: NodeStatus) {
        let mut peer_status = self.peer_status.write().await;
        peer_status.insert(id, status);
    }

    /// Returns the current status of all known peers.
    ///
    /// # Returns
    ///
    /// A map from peer IDs to their current status. Peers with no recorded
    /// status will have the default status.
    pub async fn statuses(&self) -> HashMap<PeerId, NodeStatus> {
        let peer_status = self.peer_status.read().await;
        self.peers
            .iter()
            .map(|(_addr, id)| (id.clone(), peer_status.get(id).cloned().unwrap_or_default()))
            .collect()
    }

    /// Returns an iterator over connection managers for all peers.
    ///
    /// # Returns
    ///
    /// An iterator yielding connection managers for each peer in the community.
    pub fn all_peers(&self) -> impl Iterator<Item = Arc<dyn transport::ConnectionManager<T>>> + '_ {
        self.connections.values().map(|cm| cm.clone())
    }

    /// Looks up the peer ID for a given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The network address to look up
    ///
    /// # Returns
    ///
    /// `Some(peer_id)` if the address is known, `None` otherwise.
    pub fn id_of(&self, addr: &T::Address) -> Option<PeerId> {
        self.peers.get(addr).cloned()
    }

    /// Returns the connection manager for a specific peer.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the peer to get the connection for
    ///
    /// # Returns
    ///
    /// `Some(connection_manager)` if the peer is known, `None` otherwise.
    pub fn connection(&self, id: &PeerId) -> Option<Arc<dyn transport::ConnectionManager<T>>> {
        self.connections.get(id).cloned()
    }

    /// Returns connection managers for all neighbor peers.
    ///
    /// # Returns
    ///
    /// A map from neighbor peer IDs to their connection managers.
    pub fn neighbours(&self) -> HashMap<PeerId, Arc<dyn transport::ConnectionManager<T>>> {
        self.neighbours
            .iter()
            .filter_map(|id| self.connections.get(id).map(|cm| (id.clone(), cm.clone())))
            .collect()
    }

    /// Returns the transport layer used by this community.
    ///
    /// # Returns
    ///
    /// An `Arc` pointing to the transport instance.
    pub fn transport(&self) -> Arc<T> {
        self.transport.clone()
    }

    /// Retrieves the public key for a peer, if available.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the peer whose public key to retrieve
    ///
    /// # Returns
    ///
    /// `Some(public_key)` if a key is stored for the peer, `None` otherwise.
    pub async fn pubkey(&self, id: PeerId) -> Option<PublicKey> {
        self.keys.read().await.get(&id).cloned()
    }
}
