use thiserror::Error;

/// Errors that can occur during peer communication and algorithm operations
#[derive(Debug, Error)]
pub enum PeerError {
    /// Failed to serialize message
    #[error("Failed to serialize message: {message}")]
    SerializationFailed { message: String },

    /// Failed to deserialize message
    #[error("Failed to deserialize message: {message}")]
    DeserializationFailed { message: String },

    /// Transport error (e.g., connection failed, send/receive error)
    #[error("Transport error: {message}")]
    TransportError { message: String },

    /// Unknown message type received
    #[error("Unknown message type: {message}")]
    UnknownMessageType { message: String },
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config field '{field}' is required")]
    RequiredField { field: String },
}
