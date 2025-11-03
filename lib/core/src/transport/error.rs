//! Transport layer errors.
//!
//! This module defines the error types that can occur during transport operations.

use thiserror::Error;
use tokio::sync::watch::error::SendError;

use crate::status::NodeStatus;

/// Result type for transport operations.
pub type Result<T> = std::result::Result<T, TransportError>;

/// Errors that can occur during transport operations.
///
/// This encompasses network-level errors, protocol errors, and internal
/// communication failures.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Failed to connect to the specified address.
    ///
    /// This typically occurs when the peer is not reachable or not listening.
    #[error("Failed to connect to address '{address}'")]
    ConnectionFailed { address: String },

    /// Failed to bind or listen on the specified address.
    ///
    /// This typically occurs when the address is already in use or invalid.
    #[error("Failed to listen on address '{address}'")]
    ListenFailed { address: String },

    /// The provided address has an invalid format.
    #[error("Invalid address format: {message}")]
    InvalidAddress { message: String },

    /// A network I/O error occurred.
    ///
    /// This covers low-level network errors like socket errors, read/write failures, etc.
    #[error("Network I/O error: {message}")]
    Io { message: String },

    /// Failed to parse an address string.
    #[error("Failed to parse address: {message}")]
    AddressParseError { message: String },

    /// A channel error occurred.
    ///
    /// This typically happens when internal message channels are closed unexpectedly.
    #[error("Channel error: {message}")]
    ChannelError { message: String },

    /// Failed to serialize or deserialize data.
    ///
    /// This is automatically converted from `serde_json::Error`.
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// An error occurred within the algorithm implementation.
    #[error("Algorithm error: {message}")]
    AlgorithmError { message: String },

    /// Received a message from an unknown peer.
    ///
    /// This occurs when a message is received from an address that is not
    /// registered in the community.
    #[error("Unknown peer: {addr}")]
    UnknownPeer { addr: String },

    /// The connection was closed.
    ///
    /// This occurs when attempting to use a connection that has been closed.
    #[error("Connection closed")]
    ConnectionClosed,

    /// An error occurred with a watch channel.
    ///
    /// This is automatically converted from `tokio::sync::watch::error::SendError`.
    #[error("Watch error: {0}")]
    WatchError(#[from] SendError<NodeStatus>),
}
