//! Peer proxy generation utilities.
//!
//! This module generates peer proxy types that provide type-safe methods
//! for sending messages to other nodes.

use crate::handler_parsing::HandlerInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// Generates peer methods for communicating with other nodes.
///
/// For each handler method, generates a corresponding method on the peer type
/// that serializes the message and sends it via the connection manager.
///
/// # Arguments
///
/// * `handlers` - The handler methods to generate peer methods for
pub(crate) fn generate_peer_methods(handlers: &[HandlerInfo]) -> Vec<TokenStream2> {
    handlers
        .iter()
        .map(|handler| {
            let method_name = &handler.method_name;
            let msg_type = &handler.msg_type;
            let msg_type_str = quote!(#msg_type).to_string().replace(" ", "");

            match &handler.reply_type {
                None => {
                    // Cast method (no reply expected)
                    quote! {
                        pub async fn #method_name(&self, msg: &#msg_type) -> Result<(), ::framework::PeerError> {
                            use ::framework::transport::ConnectionManager;
                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_str.to_string(),
                                ::serde_json::to_vec(msg)
                                    .map_err(|e| ::framework::PeerError::SerializationFailed {
                                        message: format!("Failed to serialize message of type '{}': {}", #msg_type_str, e)
                                    })?
                            );

                            let envelope_bytes = ::serde_json::to_vec(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            self.connection_manager.cast(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to cast message: {}", e)
                                })?;

                            Ok(())
                        }
                    }
                }
                Some(reply_type) => {
                    // Send method (expects reply)
                    let reply_type_str = quote!(#reply_type).to_string().replace(" ", "");
                    quote! {
                        pub async fn #method_name(&self, msg: &#msg_type) -> Result<#reply_type, ::framework::PeerError> {
                            use ::framework::transport::ConnectionManager;
                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_str.to_string(),
                                ::serde_json::to_vec(msg)
                                    .map_err(|e| ::framework::PeerError::SerializationFailed {
                                        message: format!("Failed to serialize message of type '{}': {}", #msg_type_str, e)
                                    })?
                            );

                            let envelope_bytes = ::serde_json::to_vec(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            let reply_bytes = self.connection_manager.send(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to send message: {}", e)
                                })?;

                            let reply: #reply_type = ::serde_json::from_slice(&reply_bytes)
                                .map_err(|e| ::framework::PeerError::DeserializationFailed {
                                    message: format!("Failed to deserialize reply of type '{}': {}", #reply_type_str, e)
                                })?;

                            Ok(reply)
                        }
                    }
                }
            }
        })
        .collect()
}

/// Generates the peer struct definition.
///
/// Creates a struct like `AlgorithmNamePeer<T>` that wraps a connection manager.
///
/// # Arguments
///
/// * `peer_name` - The name for the peer type
pub(crate) fn generate_peer_struct(peer_name: &syn::Ident) -> TokenStream2 {
    quote! {
        #[derive(Clone)]
        pub struct #peer_name<T: ::framework::transport::Transport> {
            connection_manager: ::std::sync::Arc<dyn ::framework::transport::ConnectionManager<T>>,
        }

        impl<T: ::framework::transport::Transport> #peer_name<T> {
            pub fn new(connection_manager: ::std::sync::Arc<dyn ::framework::transport::ConnectionManager<T>>) -> Self {
                Self { connection_manager }
            }
        }
    }
}
