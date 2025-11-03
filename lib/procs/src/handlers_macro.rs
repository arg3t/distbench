//! Implementation of the `#[framework::handlers]` attribute macro.
//!
//! This macro generates the AlgorithmHandler trait implementation and peer
//! method implementations from handler methods.

use crate::handler_parsing::{extract_all_handlers, generate_algorithm_handler_impl};
use crate::peer_generation::generate_peer_methods;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemImpl, Type};

/// Implements the `#[framework::handlers]` macro.
///
/// This macro:
/// - Extracts all handler methods from the impl block
/// - Generates the `AlgorithmHandler` trait implementation
/// - Generates corresponding methods on the Peer type
pub(crate) fn algorithm_handlers_impl(item: TokenStream) -> TokenStream {
    let input = match syn::parse::<ItemImpl>(item.clone()) {
        Ok(input) => input,
        Err(_) => {
            return TokenStream::from(quote! {
                compile_error!("Failed to parse item");
            });
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

    // Extract all methods as handlers
    let handlers = extract_all_handlers(&input);

    // If no handlers found, just return the original impl
    if handlers.is_empty() {
        return TokenStream::from(quote! { #input });
    }

    let algorithm_handler_impl = generate_algorithm_handler_impl(&base_ident, &handlers);
    let peer_methods = generate_peer_methods(&handlers);

    // Generate only the impl block for Peer methods, not the struct itself
    let peer_methods_impl = quote! {
        impl<T: ::framework::transport::Transport> #peer_name<T> {
            #(#peer_methods)*
        }
    };

    let expanded = quote! {
        #[allow(dead_code)]
        #input

        #algorithm_handler_impl

        #peer_methods_impl
    };

    TokenStream::from(expanded)
}
