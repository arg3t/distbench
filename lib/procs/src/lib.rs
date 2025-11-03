use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ImplItem, ImplItemFn, ItemImpl, Type};

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

struct HandlerInfo {
    method_name: syn::Ident,
    msg_type: Box<Type>,
    is_async: bool,
    reply_type: Option<Box<Type>>, // UPDATED: To store return type
}

fn extract_handlers(input: &ItemImpl) -> Vec<HandlerInfo> {
    input
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                if is_handler_method(method) {
                    return parse_handler_method(method);
                }
            }
            None
        })
        .collect()
}

fn is_handler_method(method: &ImplItemFn) -> bool {
    method
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("handler"))
}

fn parse_handler_method(method: &ImplItemFn) -> Option<HandlerInfo> {
    let method_name = method.sig.ident.clone();
    let is_async = method.sig.asyncness.is_some();

    let reply_type = match &method.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some(ty.clone()),
    };

    if let Some(syn::FnArg::Typed(pat_type)) = method.sig.inputs.iter().nth(2) {
        if let Type::Reference(type_ref) = &*pat_type.ty {
            return Some(HandlerInfo {
                method_name,
                msg_type: type_ref.elem.clone(),
                is_async,
                reply_type,
            });
        }
    }

    None
}

fn generate_peer_methods(handlers: &[HandlerInfo]) -> Vec<TokenStream2> {
    handlers
        .iter()
        .map(|handler| {
            let method_name = &handler.method_name;
            let msg_type = &handler.msg_type;
            let msg_type_id_str = quote!(stringify!(#msg_type)).to_string();

            match &handler.reply_type {
                None => {
                    quote! {
                        pub async fn #method_name(&self, msg: &#msg_type) -> Result<(), ::framework::PeerError> {
                            // Create message envelope with type ID
                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_id_str.to_string(),
                                ::serde_json::to_vec(msg)
                                    .map_err(|e| ::framework::PeerError::SerializationFailed {
                                        message: format!("Failed to serialize message of type '{}': {}", #msg_type_id_str, e)
                                    })?
                            );

                            let envelope_bytes = ::serde_json::to_vec(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            // Use cast() for fire-and-forget
                            self.connection.cast(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to cast message: {}", e)
                                })?;

                            Ok(())
                        }
                    }
                }
                Some(reply_type) => {
                    // For handlers with reply, use send()
                    let reply_type_id_str = quote!(stringify!(#reply_type)).to_string();
                    quote! {
                        pub async fn #method_name(&self, msg: &#msg_type) -> Result<#reply_type, ::framework::PeerError> {
                            // Create message envelope with type ID
                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_id_str.to_string(),
                                ::serde_json::to_vec(msg)
                                    .map_err(|e| ::framework::PeerError::SerializationFailed {
                                        message: format!("Failed to serialize message of type '{}': {}", #msg_type_id_str, e)
                                    })?
                            );

                            let envelope_bytes = ::serde_json::to_vec(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            // Use send() for request-response
                            let reply_bytes = self.connection.send(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to send message: {}", e)
                                })?;

                            let reply: #reply_type = ::serde_json::from_slice(&reply_bytes)
                                .map_err(|e| ::framework::PeerError::DeserializationFailed {
                                    message: format!("Failed to deserialize reply of type '{}': {}", #reply_type_id_str, e)
                                })?;

                            Ok(reply)
                        }
                    }
                }
            }
        })
        .collect()
}

fn generate_algorithm_handler_impl(self_ty: &Type, handlers: &[HandlerInfo]) -> TokenStream2 {
    let mut handle_arms = Vec::new();

    for handler in handlers {
        let method_name = &handler.method_name;
        let msg_type = &handler.msg_type;
        let msg_type_id_str = quote!(stringify!(#msg_type)).to_string();

        let base_call = if handler.is_async {
            quote! { self.#method_name(src.clone(), &msg).await }
        } else {
            quote! { self.#method_name(src.clone(), &msg) }
        };

        // Handle reply logic
        let call_expr = match &handler.reply_type {
            None => quote! {
                #base_call;
                return Ok(Vec::new()); // Empty response for cast messages
            },
            Some(reply_type) => {
                let reply_type_id_str = quote!(stringify!(#reply_type)).to_string();
                quote! {
                    let reply: #reply_type = #base_call;

                    let reply_bytes = ::serde_json::to_vec(&reply)
                        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                            Box::new(::framework::PeerError::SerializationFailed {
                                message: format!("Failed to serialize reply of type '{}': {}", #reply_type_id_str, e)
                            })
                        })?;

                    return Ok(reply_bytes);
                }
            }
        };

        handle_arms.push(quote! {
            if msg_type_id == #msg_type_id_str {
                let msg = ::serde_json::from_slice::<#msg_type>(&msg_bytes)
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                        Box::new(::framework::PeerError::DeserializationFailed {
                            message: format!("Failed to deserialize message of type '{}' from {:?}: {}", #msg_type_id_str, src, e)
                        })
                    })?;

                #call_expr
            }
        });
    }

    quote! {
        impl ::framework::AlgorithmHandler for #self_ty {
            async fn handle(
                &self,
                src: ::framework::community::PeerId,
                msg_type_id: String,
                msg_bytes: Vec<u8>
            ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
                #(#handle_arms)*

                Err(Box::new(::framework::PeerError::UnknownMessageType {
                    message: format!("Received unhandled message type '{}'", msg_type_id)
                }))
            }
        }
    }
}

fn generate_peer_struct(peer_name: &syn::Ident, peer_methods: &[TokenStream2]) -> TokenStream2 {
    quote! {
        #[derive(Clone)]
        pub struct #peer_name<T: ::framework::transport::Transport> {
            connection: T::Connection,
        }

        impl<T: ::framework::transport::Transport> #peer_name<T> {
            pub fn new(connection: T::Connection) -> Self {
                Self { connection }
            }

            #(#peer_methods)*
        }
    }
}

#[proc_macro_attribute]
pub fn distalg(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty;

    let self_ty_str = quote!(#self_ty).to_string().replace(' ', "");
    let peer_name = syn::Ident::new(
        &format!("{}Peer", self_ty_str),
        proc_macro2::Span::call_site(),
    );

    let handlers = extract_handlers(&input);
    let algorithm_handler_impl = generate_algorithm_handler_impl(self_ty, &handlers);
    let peer_methods = generate_peer_methods(&handlers); // UPDATED: Pass handlers
    let peer_struct = generate_peer_struct(&peer_name, &peer_methods);

    let expanded = quote! {
        #[allow(dead_code)]
        #input

        #algorithm_handler_impl

        #peer_struct
    };

    TokenStream::from(expanded)
}
