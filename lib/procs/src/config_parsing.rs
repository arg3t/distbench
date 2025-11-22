//! Configuration field parsing utilities.
//!
//! This module provides functionality for parsing `#[distbench::config]` attributes
//! on algorithm struct fields.

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields, Type};

/// Information about a configuration field.
pub(crate) struct ConfigField {
    pub field_name: syn::Ident,
    pub field_type: Type,
    pub default_value: Option<syn::Expr>,
}

/// Information about a child algorithm field.
pub(crate) struct ChildField {
    pub field_name: syn::Ident,
    pub field_type: Type,
}

/// Parses a field to check if it has a `#[distbench::config]` attribute.
///
/// Supports two forms:
/// - `#[distbench::config]` - Required field
/// - `#[distbench::config(default = value)]` - Optional field with default
///
/// # Arguments
///
/// * `field` - The field to parse
///
/// # Returns
///
/// * `Ok(Some(ConfigField))` - Field has config attribute
/// * `Ok(None)` - Field does not have config attribute
/// * `Err(syn::Error)` - Invalid config attribute format
pub(crate) fn parse_config_field(field: &Field) -> Result<Option<ConfigField>, syn::Error> {
    let Some(field_name) = &field.ident else {
        return Ok(None);
    };

    for attr in &field.attrs {
        let path_str = attr.into_token_stream().to_string().replace(" ", "");
        let is_config = path_str.contains("distbench::config");

        if is_config {
            let default_value = match &attr.meta {
                // Case 1: #[distbench::config]
                syn::Meta::Path(_) => None,

                // Case 2: #[distbench::config(...)]
                syn::Meta::List(meta_list) => {
                    if meta_list.tokens.is_empty() {
                        // Case 2a: #[distbench::config()]
                        None
                    } else {
                        // Attempt to parse the tokens as `Meta`
                        match syn::parse2::<syn::Meta>(meta_list.tokens.clone()) {
                            // Case 2b: #[distbench::config(default = ...)]
                            Ok(syn::Meta::NameValue(nv)) => {
                                if nv.path.is_ident("default") {
                                    Some(nv.value)
                                } else {
                                    // Case 2c: #[distbench::config(other = ...)] - Invalid
                                    return Err(syn::Error::new_spanned(
                                        nv.path,
                                        "Invalid config attribute. Expected `default = ...`",
                                    ));
                                }
                            }
                            // Case 2d: #[distbench::config(foo)] or other invalid Meta
                            Ok(other_meta) => {
                                return Err(syn::Error::new_spanned(
                                    other_meta,
                                    "Invalid config attribute format. Expected `#[distbench::config(default = ...)]` or `#[distbench::config]`"
                                ));
                            }
                            // Case 2e: #[distbench::config(1000)] or other invalid syntax
                            Err(_) => {
                                return Err(syn::Error::new_spanned(
                                    meta_list.tokens.clone(),
                                    "Invalid config attribute format. Expected `default = ...`",
                                ));
                            }
                        }
                    }
                }

                // Case 3: #[distbench::config = ...] - Invalid
                syn::Meta::NameValue(nv) => {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "Invalid config attribute format. Use `#[distbench::config]` or `#[distbench::config(default = ...)]`"
                    ));
                }
            };

            return Ok(Some(ConfigField {
                field_name: field_name.clone(),
                field_type: field.ty.clone(),
                default_value,
            }));
        }
    }
    Ok(None)
}

/// Parses a field to check if it has a `#[distbench::child]` attribute.
///
/// # Arguments
///
/// * `field` - The field to parse
///
/// # Returns
///
/// * `Ok(Some(ChildField))` - Field has child attribute
/// * `Ok(None)` - Field does not have child attribute
/// * `Err(syn::Error)` - Invalid child attribute format
pub(crate) fn parse_child_field(field: &Field) -> Result<Option<ChildField>, syn::Error> {
    let Some(field_name) = &field.ident else {
        return Ok(None);
    };

    for attr in &field.attrs {
        let path_str = attr.into_token_stream().to_string().replace(" ", "");
        let is_child = path_str.contains("distbench::child");

        if is_child {
            // Validate that the attribute is just #[distbench::child] with no parameters
            match &attr.meta {
                syn::Meta::Path(_) => {
                    return Ok(Some(ChildField {
                        field_name: field_name.clone(),
                        field_type: field.ty.clone(),
                    }));
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "Invalid child attribute format. Expected `#[distbench::child]` with no parameters",
                    ));
                }
            }
        }
    }
    Ok(None)
}

/// Extracts config, child, and non-config fields from a struct.
///
/// # Arguments
///
/// * `input` - The struct definition
///
/// # Returns
///
/// A tuple of (config_fields, child_fields, default_fields)
///
/// # Errors
///
/// Returns an error if any config or child attribute is malformed.
pub(crate) fn extract_fields(
    input: &DeriveInput,
) -> Result<(Vec<ConfigField>, Vec<ChildField>, Vec<Field>), syn::Error> {
    let mut config_fields = Vec::new();
    let mut child_fields = Vec::new();
    let mut default_fields = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields) = &data_struct.fields {
            for field in &fields.named {
                match parse_config_field(field)? {
                    Some(config_field) => {
                        config_fields.push(config_field);
                    }
                    None => match parse_child_field(field)? {
                        Some(child_field) => {
                            child_fields.push(child_field);
                        }
                        None => {
                            default_fields.push(field.clone());
                        }
                    },
                }
            }
        }
    }

    Ok((config_fields, child_fields, default_fields))
}

/// Helper function to extract the innermost type identifier from a type.
///
/// For example:
/// - `MyAlgorithm` -> `MyAlgorithm`
/// - `Arc<MyAlgorithm>` -> `MyAlgorithm`
/// - `std::sync::Arc<MyAlgorithm>` -> `MyAlgorithm`
fn extract_type_ident(ty: &Type) -> Option<&syn::Ident> {
    match ty {
        Type::Path(type_path) => {
            // Get the last segment of the path
            let last_segment = type_path.path.segments.last()?;

            // Check if it has angle-bracketed generic arguments
            if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                // Look for the first type argument
                for arg in &args.args {
                    if let syn::GenericArgument::Type(inner_ty) = arg {
                        // Recursively extract from the inner type
                        return extract_type_ident(inner_ty);
                    }
                }
            }

            // No generic arguments, return the identifier
            Some(&last_segment.ident)
        }
        _ => None,
    }
}

/// Generates the configuration struct for an algorithm.
///
/// Creates a struct like `AlgorithmNameConfig` with optional fields
/// for each config parameter and child algorithms.
///
/// # Arguments
///
/// * `alg_name` - The name of the algorithm
/// * `config_fields` - The configuration fields to include
/// * `child_fields` - The child algorithm fields to include
pub(crate) fn generate_config_struct(
    alg_name: &syn::Ident,
    config_fields: &[ConfigField],
    child_fields: &[ChildField],
) -> TokenStream2 {
    let config_name = syn::Ident::new(
        &format!("{}Config", alg_name),
        proc_macro2::Span::call_site(),
    );

    let child_field_defs: Vec<_> = child_fields
        .iter()
        .map(|cf| {
            let name = &cf.field_name;
            // Extract the algorithm type name and append "Config"
            let config_ty = if let Some(type_ident) = extract_type_ident(&cf.field_type) {
                let config_type_name = format!("{}Config", type_ident);
                let config_ident = syn::Ident::new(&config_type_name, type_ident.span());
                quote! { #config_ident }
            } else {
                let ty = &cf.field_type;
                quote! { #ty }
            };
            quote! { pub #name: Option<#config_ty> }
        })
        .collect();

    let field_defs: Vec<_> = config_fields
        .iter()
        .map(|cf| {
            let name = &cf.field_name;
            let ty = &cf.field_type;
            quote! { pub #name: Option<#ty> }
        })
        .collect();

    quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        pub struct #config_name {
            #(#child_field_defs,)*
            #(#field_defs),*
        }
    }
}
