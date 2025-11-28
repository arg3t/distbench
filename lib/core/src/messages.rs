//! Message types for node-to-node communication.
//!
//! This module defines the message types used for communication between nodes
//! in the distributed system.

use crate::algorithm::AlgoPath;
use rkyv::{Archive, Deserialize, Serialize};
use std::fmt::Display;

/// Messages exchanged between nodes for coordination and algorithm execution.
///
/// `NodeMessage` is an envelope type that wraps different kinds of messages
/// that nodes send to each other during execution.
#[derive(Serialize, Deserialize, Archive)]
#[archive(check_bytes)]
pub enum NodeMessage {
    /// Indicates that a node has started and is ready to participate.
    Started,

    /// Indicates that a node has finished execution.
    Finished,

    /// Requests the public key from a peer.
    AnnouncePubKey([u8; 32]),

    /// An algorithm-specific message.
    ///
    /// # Fields
    ///
    /// * `0` - The message type identifier (used for deserialization)
    /// * `1` - The serialized message payload
    Algorithm(String, Vec<u8>, AlgoPath),
}

/// Concrete packaged message, used to pass messages accross
/// layers
#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlgorithmMessage {
    pub type_id: String,
    pub bytes: Vec<u8>,
}

/// A trait for any type that can be packaged into an AlgorithmMessage.
pub trait Packagable: serde::Serialize {
    type DeserType: for<'de> serde::Deserialize<'de>;
    /// Returns the type identifier for this message.
    fn type_id() -> &'static str;
}

macro_rules! impl_packagable_with_name {
    ($($t:ty => $name:ty),*) => {
        $(
            impl Packagable for $t {
                type DeserType = $name;
                fn type_id() -> &'static str {
                    stringify!($name)
                }
            }
        )*
    };
}

macro_rules! impl_packagable_full {
    ($($t:ty),*) => {
        $(
            impl_packagable_with_name!($t => $t);
            impl_packagable_with_name!(Vec<$t> => Vec<$t>);

        )*
    };
}

impl_packagable_full!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, String);
impl_packagable_with_name!(
    &str => String,
    str => String
);

impl AlgorithmMessage {
    pub fn to_string(&self) -> String {
        format!("{{ type_id: {}, bytes: {:?} }}", self.type_id, self.bytes)
    }
}

impl Display for AlgorithmMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl AsRef<AlgorithmMessage> for AlgorithmMessage {
    fn as_ref(&self) -> &AlgorithmMessage {
        self
    }
}
