use thiserror::Error;
use tokio::sync::watch::{self, error::SendError};

use crate::NodeStatus;

pub type Result<T> = std::result::Result<T, TransportError>;

/// Errors that can occur during transport operations
#[derive(Debug, Error)]
pub enum TransportError {
    /// Failed to connect to the specified address
    #[error("Failed to connect to address '{address}'")]
    ConnectionFailed { address: String },

    /// Failed to bind/listen on the specified address
    #[error("Failed to listen on address '{address}'")]
    ListenFailed { address: String },

    /// Invalid address format
    #[error("Invalid address format: {message}")]
    InvalidAddress { message: String },

    /// Network I/O error
    #[error("Network I/O error: {message}")]
    Io { message: String },

    /// Address parsing error
    #[error("Failed to parse address: {message}")]
    AddressParseError { message: String },

    /// Channel error (e.g., sender/receiver dropped)
    #[error("Channel error: {message}")]
    ChannelError { message: String },

    // Serde error
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// Algorithm error
    #[error("Algorithm error: {message}")]
    AlgorithmError { message: String },

    /// Unknown peer
    #[error("Unknown peer: {addr}")]
    UnknownPeer { addr: String },

    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Watch error
    #[error("Watch error: {0}")]
    WatchError(#[from] SendError<NodeStatus>),
}
