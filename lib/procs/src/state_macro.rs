//! Implementation of the `#[distbench::state]` attribute macro.
//!
//! This macro transforms an algorithm state struct into a complete algorithm
//! implementation with configuration support and peer management.

use crate::config_parsing::{extract_fields, generate_config_struct, ChildField};
use crate::peer_generation::generate_peer_structs;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields};

/// Implements the `#[distbench::state]` macro.
///
/// This macro:
/// - Extracts `#[distbench::config]` fields
/// - Generates a corresponding Config struct
/// - Adds `peers` and `stopped` fields
/// - Implements the `AlgorithmFactory` trait
/// - Generates a Peer struct
/// - Implements the `SelfTerminating` trait
pub(crate) fn algorithm_state_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
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

    let config_name = syn::Ident::new(
        &format!("{}Config", alg_name_str),
        proc_macro2::Span::call_site(),
    );

    // Extract config fields, handling potential errors
    let (config_fields, child_fields, default_fields) = match extract_fields(&input) {
        Ok((config, child, default)) => (config, child, default),
        Err(e) => {
            // Convert the syn::Error into a compile_error! token stream
            return e.into_compile_error().into();
        }
    };

    // Generate config struct if there are config fields
    let config_struct = generate_config_struct(alg_name, &config_fields, &child_fields);

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
            // Remove config and child attributes from all fields
            for field in fields.named.iter_mut() {
                field.attrs.retain(|attr| {
                    let path_str = attr.into_token_stream().to_string().replace(" ", "");
                    !path_str.contains("distbench::config")
                        && !path_str.contains("distbench::child")
                });
            }

            let id_field: Field = syn::parse_quote! {
                __id: ::distbench::community::PeerId
            };
            let key_field: Field = syn::parse_quote! {
                __key: ::distbench::crypto::PrivateKey
            };
            let stopped_field: Field = syn::parse_quote! {
                #[allow(dead_code)]
                __stopped_tx: ::std::sync::Arc<::tokio::sync::watch::Sender<bool>>
            };
            let status_rx_field: Field = syn::parse_quote! {
                #[allow(dead_code)]
                __stopped_rx: ::tokio::sync::watch::Receiver<bool>
            };
            let network_size_field: Field = syn::parse_quote! {
                __network_size: u32
            };

            let connections_field: Field = syn::parse_quote! {
                __connections: ::std::collections::HashMap<::distbench::community::PeerId, #peer_name>
            };
            let keystore_field: Field = syn::parse_quote! {
                __keystore: ::distbench::community::KeyStore
            };
            let formatter_field: Field = syn::parse_quote! {
                __formatter: ::std::sync::Arc<::distbench::encoding::Formatter>
            };
            let path_field: Field = syn::parse_quote! {
                __path: ::distbench::algorithm::AlgoPath
            };
            let parent_field: Field = syn::parse_quote! {
                __parent: ::std::sync::OnceLock<::std::sync::Weak<dyn ::distbench::algorithm::DeliverableAlgorithm>>
            };
            let child_deliverables_field: Field = syn::parse_quote! {
                #[allow(dead_code)]
                __child_deliverables: ::std::sync::Mutex<::std::vec::Vec<::std::sync::Arc<dyn ::distbench::algorithm::DeliverableAlgorithm>>>
            };
            let nodeset_field: Field = syn::parse_quote! {
                __nodeset: ::distbench::node::NodeSet
            };

            fields.named.extend(vec![
                id_field,
                key_field,
                stopped_field,
                status_rx_field,
                network_size_field,
                connections_field,
                keystore_field,
                formatter_field,
                path_field,
                parent_field,
                child_deliverables_field,
                nodeset_field,
            ]);
        }
    }

    let field_inits = generate_field_initializers(&config_fields, &default_fields);
    let helper_fns = generate_helper_fns(alg_name, &peer_name);
    let factory_impl = generate_factory_impl(
        &config_name,
        alg_name,
        &peer_name,
        &field_inits,
        &child_fields,
    );
    let peer_struct = generate_peer_structs(&peer_name);
    let self_terminating_impl = generate_self_terminating_impl(alg_name);
    let algorithm_handler_impl = generate_algorithm_handler_impl(alg_name, &child_fields);
    let named_impl = generate_named_impl(alg_name);

    let expanded = quote! {
        #input

        #helper_fns

        #config_struct

        #factory_impl

        #named_impl

        #peer_struct

        #self_terminating_impl

        #algorithm_handler_impl
    };

    TokenStream::from(expanded)
}

/// Generates helper functions for the algorithm.
fn generate_helper_fns(alg_name: &syn::Ident, peer_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #alg_name {
            fn N(&self) -> u32 {
                self.__network_size + 1
            }

            fn id(&self) -> &distbench::community::PeerId {
                &self.__id
            }

            fn peers(&self) -> impl Iterator<Item = (&distbench::community::PeerId, &#peer_name)> {
                self.__connections.iter().map(|(peer_id, peer)| (peer_id, peer))
            }

            fn peer(&self, id: &distbench::community::PeerId) -> Option<#peer_name> {
                self.__connections.get(id).cloned()
            }

            fn sign<M>(&self, msg: M) -> ::distbench::signing::Signed<M>
                where M: ::distbench::signing::Digest + ::serde::Serialize + for<'de> ::serde::de::Deserialize<'de>
            {
                let signature = self.__key.sign(&msg.digest());
                let id = self.__id.clone();

                ::distbench::signing::Signed::new(msg, signature, id)
            }

            fn nodes(&self) -> ::distbench::node::NodeSet {
                self.__nodeset.clone()
            }

            pub fn set_parent(&self, parent: ::std::sync::Weak<dyn ::distbench::algorithm::DeliverableAlgorithm>) -> Result<(), ::distbench::ConfigError> {
                use ::distbench::algorithm::Named;

                self.__parent.set(parent).map_err(|_| ::distbench::ConfigError::SetParentError { child_name: self.name().to_string() })
            }

            async fn deliver(&self, src: ::distbench::community::PeerId, msg_bytes: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
                if let Some(parent) = self.__parent.get() {
                    if let Some(parent_arc) = parent.upgrade() {
                        return parent_arc.deliver(src, msg_bytes).await;
                    }
                }
                Ok(None)
            }
        }
    }
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
                quote! { #name: self.#name.ok_or(::distbench::ConfigError::RequiredField { field: #name_str.to_string() })? }
            }
        })
        .chain(default_fields.iter().map(|f| {
            let name = &f.ident;
            quote! { #name: Default::default() }
        }))
        .collect()
}

fn generate_child_field_initializers(
    alg_name: &syn::Ident,
    child_fields: &[ChildField],
) -> (Vec<TokenStream2>, Vec<TokenStream2>, Vec<TokenStream2>) {
    if child_fields.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let mut builders = Vec::new();
    let mut initializers = Vec::new();
    let mut weak_setters = Vec::new();

    for field in child_fields {
        let name = &field.field_name;
        builders.push(quote! {
            let #name = {
                let mut new_path = path.clone();
                new_path.push(stringify!(#name).to_string());
                self.#name.build(format.clone(), key.clone(), id.clone(), community.clone(), new_path)?
            };
        });
        initializers.push(quote! { #name });

        let deliverable_struct_name = syn::Ident::new(
            &format!("Deliverable{}From{}", alg_name, name),
            proc_macro2::Span::call_site(),
        );

        weak_setters.push(quote! {
            let deliverable_arc: ::std::sync::Arc<dyn ::distbench::algorithm::DeliverableAlgorithm> = ::std::sync::Arc::new(#deliverable_struct_name::new(self_arc.clone()));
            self_arc.#name.set_parent(::std::sync::Arc::downgrade(&deliverable_arc))?;
            self_arc.__child_deliverables.lock().unwrap().push(deliverable_arc);
        });
    }

    (builders, initializers, weak_setters)
}

/// Generates the Named trait implementation.
fn generate_named_impl(alg_name: &syn::Ident) -> TokenStream2 {
    let alg_name_str = alg_name.to_string();
    quote! {
        #[async_trait::async_trait]
        impl ::distbench::algorithm::Named for #alg_name {
            fn name(&self) -> &str {
                #alg_name_str
            }
        }
    }
}

/// Generates the AlgorithmFactory trait implementation.
fn generate_factory_impl(
    config_name: &syn::Ident,
    alg_name: &syn::Ident,
    peer_name: &syn::Ident,
    field_inits: &[TokenStream2],
    child_fields: &[ChildField],
) -> TokenStream2 {
    let alg_name_str = alg_name.to_string();
    let peer_name_trait = syn::Ident::new(
        &format!("{}Service", peer_name),
        proc_macro2::Span::call_site(),
    );

    let peer_name_inner = syn::Ident::new(
        &format!("{}Inner", peer_name),
        proc_macro2::Span::call_site(),
    );

    let (builders, initializers, weak_setters) =
        generate_child_field_initializers(alg_name, child_fields);

    quote! {
        impl<T, CM> ::distbench::AlgorithmFactory<T, CM> for #config_name
        where
            T: ::distbench::transport::Transport + 'static,
            CM: ::distbench::transport::ConnectionManager<T> + 'static,
        {
            type Algorithm = #alg_name;

            fn build(
                self,
                format: ::std::sync::Arc<::distbench::encoding::Formatter>,
                key: ::distbench::crypto::PrivateKey,
                id: ::distbench::community::PeerId,
                community: ::std::sync::Arc<::distbench::community::Community<T, CM>>,
                path: ::distbench::algorithm::AlgoPath,
            ) -> Result<::std::sync::Arc<Self::Algorithm>, ::distbench::ConfigError> {
                ::log::trace!("{}::build() - Building algorithm instance for node {:?}", #alg_name_str, id);
                let conn_managers = community.clone().neighbours();
                ::log::trace!("{}::build() - Creating peer proxies for {} neighbours", #alg_name_str, conn_managers.len());
                let connections: ::std::collections::HashMap<_, _> = conn_managers
                    .into_iter()
                    .map(|(peer_id, conn_manager)| {
                        ::log::trace!("{}::build() - Creating peer proxy for {:?}", #alg_name_str, peer_id);
                        (peer_id, #peer_name::new(std::sync::Arc::new(
                            Box::new(#peer_name_inner::new(
                                conn_manager,
                                format.clone(),
                                community.clone(),
                                path.clone()))
                            as Box<dyn #peer_name_trait>)))
                    })
                    .collect();

                #(#builders)*

                // Initial state is 'not stopped' (false)
                let (stopped_tx, stopped_rx) = ::tokio::sync::watch::channel(false);
                ::log::trace!("{}::build() - Algorithm instance built successfully", #alg_name_str);

                let self_arc = ::std::sync::Arc::new(Self::Algorithm {
                    #(#field_inits,)*
                    #(#initializers,)*
                    __connections: connections,
                    __network_size: community.size() as u32,
                    __keystore: community.keystore(),
                    __stopped_tx: ::std::sync::Arc::new(stopped_tx),
                    __stopped_rx: stopped_rx,
                    __formatter: format,
                    __key: key,
                    __id: id,
                    __path: path,
                    __parent: ::std::sync::OnceLock::new(),
                    __nodeset: community.nodeset(),
                    __child_deliverables: ::std::sync::Mutex::new(::std::vec::Vec::new()),
                });

                #(#weak_setters)*

                Ok(self_arc)
            }
        }
    }
}

/// Generates the SelfTerminating trait implementation.
fn generate_self_terminating_impl(alg_name: &syn::Ident) -> TokenStream2 {
    let alg_name_str = alg_name.to_string();
    quote! {
        #[async_trait::async_trait]
        impl ::distbench::SelfTerminating for #alg_name {
            async fn terminate(&self) {
                ::log::trace!("{}.terminate() - Sending termination signal", #alg_name_str);
                // Send 'true' to signal stopped.
                // Ignore result: Err means all receivers dropped.
                let _ = self.__stopped_tx.send(true);
                ::log::trace!("{}.terminate() - Termination signal sent", #alg_name_str);
            }

            async fn terminated(&self) -> bool {
                let mut rx = self.__stopped_rx.clone();
                // wait_for checks current value, and if not true,
                // awaits until the predicate (*stopped) is true.
                // Err means sender was dropped, which is fine.
                let _ = rx.wait_for(|stopped| *stopped).await;
                true
            }
        }
    }
}

/// Generate the message handler block that calls the respective algorithm's handler.
fn generate_algorithm_handler_impl(
    self_ty: &syn::Ident,
    child_fields: &[ChildField],
) -> TokenStream2 {
    // Generate match arms for routing to child algorithms
    let child_routing = if !child_fields.is_empty() {
        let match_arms: Vec<_> = child_fields.iter().map(|field| {
            let field_name = &field.field_name;
            let field_name_str = field_name.to_string();
            quote! {
                #field_name_str => {
                    ::log::trace!("AlgorithmHandler::handle - Routing to child algorithm: {}", #field_name_str);
                    return self.#field_name.handle(src, msg_type_id, msg_bytes, &path[1..]).await;
                }
            }
        }).collect();

        quote! {
            if !path.is_empty() {
                let child_name = &path[0];
                ::log::trace!("AlgorithmHandler::handle - Path not empty, routing to child: {}", child_name);
                match child_name.as_str() {
                    #(#match_arms)*
                    _ => {
                        return Err(::distbench::PeerError::UnknownChild {
                            child: child_name.clone()
                        }.into());
                    }
                }
            }
        }
    } else {
        quote! {
            if !path.is_empty() {
                return Err(::distbench::PeerError::NoChildAlgorithms {
                }.into());
            }
        }
    };

    quote! {
        #[async_trait::async_trait]
        impl ::distbench::AlgorithmHandler for #self_ty {
            async fn handle(
                &self,
                src: ::distbench::community::PeerId,
                msg_type_id: String,
                msg_bytes: Vec<u8>,
                path: &[String],
            ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
                ::log::trace!("AlgorithmHandler::handle - Received message type '{}' from {:?}, path: {:?}", msg_type_id, src, path);

                #child_routing

                // If we reach here, path is empty, so handle locally
                ::log::trace!("AlgorithmHandler::handle - Handling message locally");
                self.handle_msg(src, msg_type_id, msg_bytes).await
            }
        }
    }
}
