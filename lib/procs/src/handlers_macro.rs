//! Implementation of the `#[distbench::handlers]` attribute macro.
//!
//! This macro generates the AlgorithmHandler trait implementation and peer
//! method implementations from handler methods.

use crate::handler_parsing::{extract_all_handlers, generate_algorithm_handler_impl};
use crate::peer_generation::{
    generate_peer_ergonomic_fns, generate_peer_methods, generate_peer_trait_fns,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse::Parser, ItemImpl, Token, Type};

/// Implements the `#[distbench::handlers]` macro.
///
/// This macro:
/// - Extracts all handler methods from the impl block
/// - Generates the `AlgorithmHandler` trait implementation
/// - Generates corresponding methods on the Peer type
/// - If `from = ...` is specified, generates a DeliverableAlgorithm implementation
pub(crate) fn algorithm_handlers_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the optional 'from' argument
    let from_field: Option<syn::Ident> = if !attr.is_empty() {
        match parse_from_attr(attr) {
            Ok(ident) => Some(ident),
            Err(e) => return TokenStream::from(e.to_compile_error()),
        }
    } else {
        None
    };
    let input = match syn::parse::<ItemImpl>(item) {
        Ok(input) => input,
        Err(e) => {
            return TokenStream::from(e.to_compile_error());
        }
    };

    let self_ty = &input.self_ty;

    let base_ident = match &**self_ty {
        Type::Path(syn::TypePath { qself: None, path }) => {
            if let Some(segment) = path.segments.last() {
                segment.ident.clone()
            } else {
                return TokenStream::from(quote! {
                    compile_error!("Could not extract a name from the implemented type.");
                });
            }
        }
        _ => {
            return TokenStream::from(quote! {
                compile_error!("The item is implemented for a type that does not have a simple name (e.g., references, arrays).");
            });
        }
    };

    let peer_name = syn::Ident::new(
        &format!("{}Peer", base_ident),
        proc_macro2::Span::call_site(),
    );

    let peer_name_inner = syn::Ident::new(
        &format!("{}Inner", peer_name),
        proc_macro2::Span::call_site(),
    );

    let peer_name_service = syn::Ident::new(
        &format!("{}Service", peer_name),
        proc_macro2::Span::call_site(),
    );

    // Extract all methods as handlers
    let handlers = match extract_all_handlers(&input) {
        Ok(handlers) => handlers,
        Err(e) => {
            return e.to_compile_error().into();
        }
    };

    // Generate either regular AlgorithmHandler or DeliverableAlgorithm based on 'from' parameter
    let handler_impl = if let Some(from_field_name) = from_field {
        // Generate DeliverableAlgorithm implementation
        generate_deliverable_impl(&base_ident, &from_field_name, &handlers)
    } else {
        // Generate regular AlgorithmHandler implementation
        let algorithm_handler_impl = generate_algorithm_handler_impl(&base_ident, &handlers);
        let peer_methods = generate_peer_methods(&handlers);
        let peer_methods_ergonomic = generate_peer_ergonomic_fns(&handlers);
        let peer_trait_fns = generate_peer_trait_fns(&handlers);

        let peer_trait_impl = quote! {
            #[async_trait::async_trait]
            trait #peer_name_service: Send + Sync {
                #(#peer_trait_fns)*
            }
        };

        // Generate only the impl block for Peer methods, not the struct itself
        let peer_methods_impl = quote! {
            #[async_trait::async_trait]
            impl<T, CM> #peer_name_service for #peer_name_inner<T, CM>
                where
                T: ::distbench::transport::Transport,
                CM: ::distbench::transport::ConnectionManager<T>
            {
                #(#peer_methods)*
            }
        };

        let peer_methods_ergonomic_impl = quote! {
            impl #peer_name
            {
                #(#peer_methods_ergonomic)*
            }
        };

        quote! {
            #algorithm_handler_impl

            #peer_methods_ergonomic_impl

            #peer_trait_impl

            #peer_methods_impl
        }
    };

    let expanded = quote! {
        #[allow(dead_code)]
        #input

        #handler_impl
    };

    TokenStream::from(expanded)
}

/// Generates a DeliverableAlgorithm implementation for the algorithm.
///
/// This creates a struct that wraps an Arc to the algorithm and implements
/// the DeliverableAlgorithm trait to handle messages from a parent algorithm.
fn generate_deliverable_impl(
    alg_name: &syn::Ident,
    from_field: &syn::Ident,
    handlers: &[crate::handler_parsing::HandlerInfo],
) -> TokenStream2 {
    // Use the field name directly
    let from_name = from_field.to_string();

    let deliverable_struct_name = syn::Ident::new(
        &format!("Deliverable{}From{}", alg_name, from_name),
        proc_macro2::Span::call_site(),
    );

    // Generate handling arms for all handlers
    let mut handle_arms = Vec::new();

    for handler in handlers {
        let method_name = &handler.method_name;
        let msg_type = &handler.msg_type;
        let msg_type_str = quote!(#msg_type).to_string().replace(" ", "");

        let base_call = if handler.is_async {
            quote! { self.algorithm.#method_name(src.clone(), &msg).await }
        } else {
            quote! { self.algorithm.#method_name(src.clone(), &msg) }
        };

        // Handle reply logic
        let call_expr = match &handler.reply_type {
            None => quote! {
                ::log::trace!("DeliverableAlgorithm::deliver - Calling handler {} (cast)", #msg_type_str);
                #base_call;
                ::log::trace!("DeliverableAlgorithm::deliver - Handler {} completed", #msg_type_str);
                return Ok(None); // Empty response for cast messages
            },
            Some(reply_type) => {
                let reply_type_str = quote!(#reply_type).to_string().replace(" ", "");
                quote! {
                    ::log::trace!("DeliverableAlgorithm::deliver - Calling handler {} (send/reply)", #msg_type_str);
                    let reply: #reply_type = #base_call;

                    ::log::trace!("DeliverableAlgorithm::deliver - Serializing reply of type {}", #reply_type_str);
                    let reply_bytes = self.algorithm.__formatter.serialize(&reply)
                        .map_err(|e| ::distbench::PeerError::SerializationFailed {
                            message: format!("Failed to serialize reply of type '{}': {}", #reply_type_str, e)
                        })?;

                    ::log::trace!("DeliverableAlgorithm::deliver - Handler {} completed, reply: {} bytes", #msg_type_str, reply_bytes.len());
                    return Ok(Some(reply_bytes));
                }
            }
        };

        handle_arms.push(quote! {
            if msg_type_id == #msg_type_str {
                ::log::trace!("DeliverableAlgorithm::deliver - Deserializing message of type {} from {:?}", #msg_type_str, src);
                let msg = self.algorithm.__formatter.deserialize::<#msg_type>(&msg_bytes)
                    .map_err(|e| ::distbench::PeerError::DeserializationFailed {
                        message: format!("Failed to deserialize message of type '{}' from {:?}: {}", #msg_type_str, src, e)
                    })?;
                let msg = msg.verify(&self.algorithm.__keystore)?;

                #call_expr
            }
        });
    }

    quote! {
        pub struct #deliverable_struct_name {
            algorithm: ::std::sync::Arc<#alg_name>,
        }

        impl #deliverable_struct_name {
            pub fn new(algorithm: ::std::sync::Arc<#alg_name>) -> Self {
                Self { algorithm }
            }
        }

        #[async_trait::async_trait]
        impl ::distbench::algorithm::DeliverableAlgorithm for #deliverable_struct_name {
            async fn deliver(
                &self,
                src: ::distbench::community::PeerId,
                msg_bytes: Vec<u8>,
            ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
                use ::distbench::Format;
                use ::distbench::signing::Verifiable;

                // First, deserialize to (String, Vec<u8>)
                let (msg_type_id, msg_bytes): (String, Vec<u8>) = self.algorithm.__formatter
                    .deserialize(&msg_bytes)
                    .map_err(|e| ::distbench::PeerError::DeserializationFailed {
                        message: format!("Failed to deserialize message envelope: {}", e)
                    })?;

                ::log::trace!("DeliverableAlgorithm::deliver - Received message of type '{}' from {:?}", msg_type_id, src);

                // Then dispatch to the appropriate handler
                #(#handle_arms)*

                Err(::distbench::PeerError::UnknownMessageType {
                    message: format!("Received unhandled message type '{}'", msg_type_id)
                }.into())
            }
        }
    }
}

/// Parses the `from = field_name` attribute argument.
fn parse_from_attr(attr: TokenStream) -> Result<syn::Ident, syn::Error> {
    let parser = |input: syn::parse::ParseStream| {
        let ident: syn::Ident = input.parse()?;
        if ident != "from" {
            return Err(syn::Error::new_spanned(ident, "Expected 'from'"));
        }
        input.parse::<Token![=]>()?;
        input.parse()
    };

    parser.parse(attr)
}
