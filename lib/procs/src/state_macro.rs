//! Implementation of the `#[framework::state]` attribute macro.
//!
//! This macro transforms an algorithm state struct into a complete algorithm
//! implementation with configuration support and peer management.

use crate::config_parsing::{extract_fields, generate_config_struct};
use crate::peer_generation::generate_peer_struct;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields};

/// Implements the `#[framework::state]` macro.
///
/// This macro:
/// - Extracts `#[framework::config]` fields
/// - Generates a corresponding Config struct
/// - Adds `peers` and `stopped` fields
/// - Implements the `AlgorithmFactory` trait
/// - Generates a Peer struct
/// - Implements the `SelfTerminating` trait
pub(crate) fn algorithm_state_impl(item: TokenStream) -> TokenStream {
    let mut input = match syn::parse::<DeriveInput>(item.clone()) {
        Ok(input) => input,
        Err(_) => {
            // Return original item if parsing fails (better for IDE)
            return item;
        }
    };
    let alg_name = &input.ident;

    let alg_name_str = alg_name.to_string();
    let peer_name = syn::Ident::new(
        &format!("{}Peer", alg_name_str),
        proc_macro2::Span::call_site(),
    );
    let peer_name_impl = syn::Ident::new(
        &format!("{}PeerImpl", alg_name_str),
        proc_macro2::Span::call_site(),
    );

    let config_name = syn::Ident::new(
        &format!("{}Config", alg_name_str),
        proc_macro2::Span::call_site(),
    );

    // Extract config fields, handling potential errors
    let (config_fields, default_fields) = match extract_fields(&input) {
        Ok((config, default)) => (config, default),
        Err(e) => {
            // Convert the syn::Error into a compile_error! token stream
            return e.into_compile_error().into();
        }
    };

    // Generate config struct if there are config fields
    let config_struct = generate_config_struct(alg_name, &config_fields);

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
            // Remove config attributes from all fields
            for field in fields.named.iter_mut() {
                field.attrs.retain(|attr| {
                    let path_str = attr.into_token_stream().to_string().replace(" ", "");
                    !path_str.contains("framework::config")
                });
            }

            let peers_field: Field = syn::parse_quote! {
                peers: ::std::collections::HashMap<::framework::community::PeerId, std::sync::Arc<Box<dyn #peer_name>>>
            };
            fields.named.push(peers_field);

            // Add the stopped field to the struct definition
            let stopped_field: Field = syn::parse_quote! {
                #[allow(dead_code)]
                stopped: ::std::sync::Arc<::tokio::sync::watch::Sender<bool>>
            };
            fields.named.push(stopped_field);

            // Add the status_rx field to the struct definition
            let status_rx_field: Field = syn::parse_quote! {
                #[allow(dead_code)]
                status_rx: ::std::sync::Arc<::tokio::sync::Mutex<::tokio::sync::watch::Receiver<bool>>>
            };
            fields.named.push(status_rx_field);
        }
    }

    let field_inits = generate_field_initializers(&config_fields, &default_fields);
    let factory_impl = generate_factory_impl(
        &config_name,
        alg_name,
        &peer_name,
        &peer_name_impl,
        &field_inits,
    );
    let peer_struct = generate_peer_struct(&peer_name_impl);
    let self_terminating_impl = generate_self_terminating_impl(alg_name);

    let expanded = quote! {
        #input

        #config_struct

        #factory_impl

        #peer_struct

        #self_terminating_impl
    };

    TokenStream::from(expanded)
}

/// Generates field initializers for the AlgorithmFactory impl.
fn generate_field_initializers(
    config_fields: &[crate::config_parsing::ConfigField],
    default_fields: &[Field],
) -> Vec<TokenStream2> {
    config_fields
        .iter()
        .map(|cf| {
            let name = &cf.field_name;
            let default = &cf.default_value;
            if let Some(default) = default {
                quote! { #name: self.#name.unwrap_or(#default) }
            } else {
                let name_str = name.to_string();
                quote! { #name: self.#name.ok_or(::framework::ConfigError::RequiredField { field: #name_str.to_string() })? }
            }
        })
        .chain(default_fields.iter().map(|f| {
            let name = &f.ident;
            quote! { #name: Default::default() }
        }))
        .collect()
}

/// Generates the AlgorithmFactory trait implementation.
fn generate_factory_impl(
    config_name: &syn::Ident,
    alg_name: &syn::Ident,
    peer_name: &syn::Ident,
    peer_name_impl: &syn::Ident,
    field_inits: &[TokenStream2],
) -> TokenStream2 {
    quote! {
        impl<F, T, CM> ::framework::AlgorithmFactory<F, T, CM> for #config_name
        where
            T: ::framework::transport::Transport + 'static,
            CM: ::framework::transport::ConnectionManager<T> + 'static,
            F: ::framework::Format + 'static,
        {
            type Algorithm = #alg_name;

            fn build(
                self,
                format: ::std::sync::Arc<F>,
                community: &::framework::community::Community<T, CM>,
            ) -> Result<::std::sync::Arc<Self::Algorithm>, ::framework::ConfigError> {
                let conn_managers = community.neighbours();
                let peers: ::std::collections::HashMap<::framework::community::PeerId, std::sync::Arc<Box<dyn #peer_name>>> = conn_managers
                    .into_iter()
                    .map(|(peer_id, conn_manager)| {
                        (peer_id, std::sync::Arc::new(Box::new(#peer_name_impl::new(conn_manager, format.clone())) as Box<dyn #peer_name>))
                    })
                    .collect();

                // Initial state is 'not stopped' (false)
                let (stopped_tx, stopped_rx) = ::tokio::sync::watch::channel(false);

                Ok(::std::sync::Arc::new(Self::Algorithm {
                    #(#field_inits,)*
                    peers,
                    stopped: ::std::sync::Arc::new(stopped_tx),
                    status_rx: ::std::sync::Arc::new(::tokio::sync::Mutex::new(stopped_rx)),
                }))
            }
        }
    }
}

/// Generates the SelfTerminating trait implementation.
fn generate_self_terminating_impl(alg_name: &syn::Ident) -> TokenStream2 {
    quote! {
        #[async_trait::async_trait]
        impl ::framework::SelfTerminating for #alg_name {
            async fn terminate(&self) {
                // Send 'true' to signal stopped.
                // Ignore result: Err means all receivers dropped.
                let _ = self.stopped.send(true);
            }

            async fn terminated(&self) -> bool {
                let mut rx = self.status_rx.lock().await;
                // wait_for checks current value, and if not true,
                // awaits until the predicate (*stopped) is true.
                // Err means sender was dropped, which is fine.
                let _ = rx.wait_for(|stopped| *stopped).await;
                true
            }
        }
    }
}
