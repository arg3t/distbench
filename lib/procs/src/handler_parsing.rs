//! Handler method parsing utilities.
//!
//! This module provides functionality for parsing handler methods from algorithm
//! implementation blocks.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ImplItem, ImplItemFn, ItemImpl, Type};

/// Information about a message handler method.
pub(crate) struct HandlerInfo {
    pub method_name: syn::Ident,
    pub msg_type: Box<Type>,
    pub is_async: bool,
    pub reply_type: Option<Box<Type>>,
}

/// Parses a method to extract handler information.
///
/// Returns `Some(HandlerInfo)` if the method matches the handler pattern:
/// - Takes 3 parameters (self, src, &MessageType)
/// - Third parameter is a reference type
///
/// # Arguments
///
/// * `method` - The method to parse
pub(crate) fn parse_handler_method(method: &ImplItemFn) -> Result<HandlerInfo, syn::Error> {
    let method_name = method.sig.ident.clone();
    let is_async = method.sig.asyncness.is_some();

    let reply_type = match &method.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some(ty.clone()),
    };

    if let Some(syn::FnArg::Typed(pat_type)) = method.sig.inputs.iter().nth(2) {
        if let Type::Reference(type_ref) = &*pat_type.ty {
            return Ok(HandlerInfo {
                method_name,
                msg_type: type_ref.elem.clone(),
                is_async,
                reply_type,
            });
        } else {
            return Err(syn::Error::new_spanned(
                pat_type,
                "The third parameter must be a reference type",
            ));
        }
    } else {
        return Err(syn::Error::new_spanned(
            method.sig.inputs.iter().nth(2).unwrap(),
            "The third parameter must be a reference type",
        ));
    }
}

/// Extracts all handler methods from an implementation block.
///
/// # Arguments
///
/// * `input` - The implementation block to parse
///
/// # Returns
///
/// A vector of `HandlerInfo` for all methods that match the handler pattern.
pub(crate) fn extract_all_handlers(input: &ItemImpl) -> Result<Vec<HandlerInfo>, syn::Error> {
    let mut errors = Vec::new();
    let mut handlers = Vec::new();
    for item in input.items.iter() {
        if let ImplItem::Fn(method) = item {
            match parse_handler_method(method) {
                Ok(handler) => handlers.push(handler),
                Err(e) => errors.push(e),
            }
        }
    }

    if errors.is_empty() {
        Ok(handlers)
    } else {
        let mut error = errors.pop().unwrap();
        for e in errors {
            error.combine(e);
        }

        Err(error)
    }
}

/// Generates the `AlgorithmHandler` trait implementation.
///
/// Creates code that dispatches incoming messages to the appropriate handler
/// methods based on message type ID.
///
/// # Arguments
///
/// * `self_ty` - The name of the algorithm type
/// * `handlers` - The list of handler methods
pub(crate) fn generate_algorithm_handler_impl(
    self_ty: &syn::Ident,
    handlers: &[HandlerInfo],
) -> TokenStream2 {
    let mut handle_arms = Vec::new();

    for handler in handlers {
        let method_name = &handler.method_name;
        let msg_type = &handler.msg_type;
        let msg_type_str = quote!(#msg_type).to_string().replace(" ", "");

        let base_call = if handler.is_async {
            quote! { self.#method_name(src.clone(), &msg).await }
        } else {
            quote! { self.#method_name(src.clone(), &msg) }
        };

        // Handle reply logic
        let call_expr = match &handler.reply_type {
            None => quote! {
                #base_call;
                return Ok(None); // Empty response for cast messages
            },
            Some(reply_type) => {
                let reply_type_str = quote!(#reply_type).to_string().replace(" ", "");
                quote! {
                    let reply: #reply_type = #base_call;

                    let reply_bytes = ::serde_json::to_vec(&reply)
                        .map_err(|e| ::framework::PeerError::SerializationFailed {
                            message: format!("Failed to serialize reply of type '{}': {}", #reply_type_str, e)
                        })?;

                    return Ok(Some(reply_bytes));
                }
            }
        };

        handle_arms.push(quote! {
            if msg_type_id == #msg_type_str {
                let msg = ::serde_json::from_slice::<#msg_type>(&msg_bytes)
                    .map_err(|e| ::framework::PeerError::DeserializationFailed {
                        message: format!("Failed to deserialize message of type '{}' from {:?}: {}", #msg_type_str, src, e)
                    })?;

                #call_expr
            }
        });
    }

    quote! {
        #[async_trait::async_trait]
        impl ::framework::AlgorithmHandler for #self_ty {
            async fn handle(
                &self,
                src: ::framework::community::PeerId,
                msg_type_id: String,
                msg_bytes: Vec<u8>
            ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
                #(#handle_arms)*

                Err(::framework::PeerError::UnknownMessageType {
                    message: format!("Received unhandled message type '{}'", msg_type_id)
                }.into())
            }
        }
    }
}
