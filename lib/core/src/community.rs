use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{transport, NodeStatus};

pub type PublicKey = Vec<u8>;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct PeerId(String);

impl PeerId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

pub struct Community<T: transport::Transport> {
    neighbours: HashSet<PeerId>,
    connections: HashMap<PeerId, Arc<dyn transport::ConnectionManager<T>>>,
    peers: HashMap<T::Address, PeerId>,
    transport: Arc<T>,
    peer_status: RwLock<HashMap<PeerId, NodeStatus>>,
    keys: RwLock<HashMap<PeerId, PublicKey>>,
}

impl<T: transport::Transport + 'static> Community<T> {
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

    pub async fn set_status(&self, id: PeerId, status: NodeStatus) {
        let mut peer_status = self.peer_status.write().await;
        peer_status.insert(id, status);
    }

    pub async fn statuses(&self) -> HashMap<PeerId, NodeStatus> {
        let peer_status = self.peer_status.read().await;
        self.peers
            .iter()
            .map(|(addr, id)| (id.clone(), peer_status.get(id).cloned().unwrap_or_default()))
            .collect()
    }

    pub fn all_peers(&self) -> impl Iterator<Item = Arc<dyn transport::ConnectionManager<T>>> + '_ {
        self.connections.values().map(|cm| cm.clone())
    }

    pub fn id_of(&self, addr: &T::Address) -> Option<PeerId> {
        self.peers.get(addr).cloned()
    }

    pub fn connection(&self, id: &PeerId) -> Option<Arc<dyn transport::ConnectionManager<T>>> {
        self.connections.get(id).cloned()
    }

    pub fn neighbours(&self) -> HashMap<PeerId, Arc<dyn transport::ConnectionManager<T>>> {
        self.neighbours
            .iter()
            .filter_map(|id| self.connections.get(id).map(|cm| (id.clone(), cm.clone())))
            .collect()
    }

    pub fn transport(&self) -> Arc<T> {
        self.transport.clone()
    }

    pub async fn pubkey(&self, id: PeerId) -> Option<PublicKey> {
        self.keys.read().await.get(&id).cloned()
    }
}
