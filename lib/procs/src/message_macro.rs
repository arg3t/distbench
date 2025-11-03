//! Implementation of the `#[framework::message]` attribute macro.
//!
//! This macro automatically derives common traits for message types.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Implements the `#[framework::message]` macro.
///
/// Automatically adds `#[derive(Serialize, Deserialize, Clone, Debug)]`
/// to the message type.
pub(crate) fn message_impl(item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);

    let new_attr_tokens =
        syn::parse_quote!(#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]);
    input.attrs.insert(0, new_attr_tokens);

    let output = quote! {
        #input
    };

    output.into()
}
