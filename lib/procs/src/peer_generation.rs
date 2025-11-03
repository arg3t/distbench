//! Peer proxy generation utilities.
//!
//! This module generates peer proxy types that provide type-safe methods
//! for sending messages to other nodes.

use crate::handler_parsing::HandlerInfo;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn generate_peer_trait_fns(handlers: &[HandlerInfo]) -> Vec<TokenStream2> {
    handlers
        .iter()
        .map(|handler| {
            let method_name = &handler.method_name;
            let msg_type = &handler.msg_type;
            let reply_type = &handler.reply_type;
            if handler.reply_type.is_some() {
                quote! {
                    async fn #method_name(&self, msg: &#msg_type) -> Result<#reply_type, ::framework::PeerError>;
                }
            } else {
                quote! {
                    async fn #method_name(&self, msg: &#msg_type) -> Result<(), ::framework::PeerError>;
                }
            }
        })
        .collect()
}

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
                        async fn #method_name(&self, msg: &#msg_type) -> Result<(), ::framework::PeerError> {
                            use ::framework::transport::ConnectionManager;
                            use ::framework::Format;

                            let msg_bytes = self.format.serialize(msg)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize message of type '{}': {}", #msg_type_str, e)
                                })?;

                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_str.to_string(),
                                msg_bytes
                            );

                            let envelope_bytes = self.format.serialize(&envelope)
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
                        async fn #method_name(&self, msg: &#msg_type) -> Result<#reply_type, ::framework::PeerError> {
                            use ::framework::transport::ConnectionManager;
                            use ::framework::Format;

                            let msg_bytes = self.format.serialize(msg)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize message of type '{}': {}", #msg_type_str, e)
                                })?;

                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_str.to_string(),
                                msg_bytes
                            );

                            let envelope_bytes = self.format.serialize(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            let reply_bytes = self.connection_manager.send(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to send message: {}", e)
                                })?;

                            let reply: #reply_type = self.format.deserialize(&reply_bytes)
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
/// Creates a struct like `AlgorithmNamePeer<T, CM>` that wraps a connection manager and format.
///
/// # Arguments
///
/// * `peer_name` - The name for the peer type
pub(crate) fn generate_peer_struct(peer_name: &syn::Ident) -> TokenStream2 {
    quote! {
        #[derive(Clone)]
        struct #peer_name<F: ::framework::Format, T: ::framework::transport::Transport, CM: ::framework::transport::ConnectionManager<T>> {
            connection_manager: ::std::sync::Arc<CM>,
            format: ::std::sync::Arc<F>,
            _phantom: ::std::marker::PhantomData<T>,
        }

        impl<F: ::framework::Format, T: ::framework::transport::Transport, CM: ::framework::transport::ConnectionManager<T>> #peer_name<F, T, CM> {
            pub fn new(connection_manager: ::std::sync::Arc<CM>, format: ::std::sync::Arc<F>) -> Self {
                Self {
                    connection_manager,
                    format,
                    _phantom: ::std::marker::PhantomData,
                }
            }
        }
    }
}
