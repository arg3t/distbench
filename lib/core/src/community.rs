use std::collections::{HashMap, HashSet};

use crate::transport;

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
    addresses: HashMap<PeerId, T::Address>,
    peers: HashMap<T::Address, PeerId>,
    keys: HashMap<PeerId, PublicKey>,
}

impl<T: transport::Transport> Community<T> {
    pub fn new(neighbours: HashSet<PeerId>, addresses: HashMap<PeerId, T::Address>) -> Self {
        Self {
            neighbours,
            peers: addresses
                .iter()
                .map(|(id, addr)| (addr.clone(), id.clone()))
                .collect(),
            addresses,
            keys: Default::default(),
        }
    }

    pub fn peer(&self, addr: &T::Address) -> Option<PeerId> {
        self.peers.get(addr).cloned()
    }

    pub fn address(&self, id: &PeerId) -> Option<T::Address> {
        self.addresses.get(id).cloned()
    }

    pub fn neighbours(&self) -> &HashSet<PeerId> {
        &self.neighbours
    }

    pub fn pubkey(&self, id: PeerId) -> Option<PublicKey> {
        self.keys.get(&id).cloned()
    }
}
