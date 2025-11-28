//! Implementation of the `#[distbench::interface]` attribute macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, FnArg, ImplItem, ItemImpl, ReturnType, Visibility};

pub(crate) fn interface_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = match syn::parse::<ItemImpl>(item.clone()) {
        Ok(input) => input,
        Err(_) => return item,
    };

    for item in &mut input.items {
        if let ImplItem::Fn(method) = item {
            if let Err(e) = transform_method(method) {
                return e.into_compile_error().into();
            }
        }
    }

    TokenStream::from(quote! { #input })
}

fn transform_method(method: &mut syn::ImplItemFn) -> syn::Result<()> {
    // 1. Ensure pub
    if !matches!(method.vis, Visibility::Public(_)) {
        return Err(syn::Error::new_spanned(
            &method.sig.ident,
            "Interface methods must be public",
        ));
    }

    // 2. Ensure &self is first argument
    let inputs = &method.sig.inputs;
    if inputs.is_empty() {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "Interface methods must have &self as first argument",
        ));
    }

    // Check self
    match &inputs[0] {
        FnArg::Receiver(_) => {}
        _ => {
            return Err(syn::Error::new_spanned(
                &inputs[0],
                "First argument must be &self",
            ));
        }
    }

    // Collect all arguments after &self
    let original_args: Vec<_> = inputs.iter().skip(2).cloned().collect();

    // 3. Transform signature
    // Add generic parameter M: ::distbench::messages::Packagable
    // Prepend __pkg_msg: &M to the argument list

    let original_ret = method.sig.output.clone();
    let ret_type = match original_ret {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    method.sig.generics = parse_quote! { <M: ::distbench::messages::Packagable> };

    // Update inputs: &self, __pkg_msg: &M, [original args...]
    method.sig.inputs = parse_quote! {
        &self, __pkg_msg: &M, #(#original_args),*
    };

    // Update return type
    method.sig.output = parse_quote! {
        -> Result<#ret_type, ::distbench::FormatError>
    };

    // 4. Transform body
    let original_block = &method.block;
    let is_async = method.sig.asyncness.is_some();

    // We inject serialization logic.
    let serialization_logic = quote! {
        let inner_bytes = self.__formatter.serialize(__pkg_msg)?;
        let alg_msg = ::distbench::messages::AlgorithmMessage {
            type_id: M::type_id().to_string(),
            bytes: inner_bytes,
        };
        let msg = self.__formatter.serialize(&alg_msg)?;
    };

    if is_async {
        // For async functions, we use an async block.
        // Return statements inside the async block return from the block, which works perfectly.
        method.block = parse_quote! {
            {
                #serialization_logic
                Ok(async move {
                    #original_block
                }.await)
            }
        };
    } else {
        // For synchronous functions, we use an IIFE closure to handle return statements.
        method.block = parse_quote! {
            {
                #serialization_logic
                let logic = || -> #ret_type {
                    #original_block
                };
                Ok(logic())
            }
        };
    }

    Ok(())
}
