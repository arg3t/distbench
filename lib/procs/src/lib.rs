use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, ImplItem, ImplItemFn, ItemImpl, Type,
};

struct HandlerInfo {
    method_name: syn::Ident,
    msg_type: Box<Type>,
    is_async: bool,
    reply_type: Option<Box<Type>>, // UPDATED: To store return type
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

fn generate_algorithm_handler_impl(self_ty: &syn::Ident, handlers: &[HandlerInfo]) -> TokenStream2 {
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

                    // '?' operator will automatically box the PeerError
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
                // '?' operator will automatically box the PeerError
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
        impl<T: ::framework::transport::Transport> ::framework::AlgorithmHandler for #self_ty<T> {
            async fn handle(
                &self,
                src: ::framework::community::PeerId,
                msg_type_id: String,
                msg_bytes: Vec<u8>
            ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
                #(#handle_arms)*

                // Use .into() to convert the concrete error to the Box<dyn Error>
                Err(::framework::PeerError::UnknownMessageType {
                    message: format!("Received unhandled message type '{}'", msg_type_id)
                }.into())
            }
        }
    }
}

// Helper struct to store config field information
struct ConfigField {
    field_name: syn::Ident,
    field_type: Type,
    default_value: Option<syn::Expr>,
}

// ### MODIFIED FUNCTION ###
// Now returns a Result to propagate errors
fn extract_fields(input: &DeriveInput) -> Result<(Vec<ConfigField>, Vec<Field>), syn::Error> {
    let mut config_fields = Vec::new();
    let mut default_fields = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields) = &data_struct.fields {
            for field in &fields.named {
                // Use ? to propagate the error
                match parse_config_field(field)? {
                    Some(config_field) => {
                        config_fields.push(config_field);
                    }
                    None => {
                        default_fields.push(field.clone());
                    }
                }
            }
        }
    }

    Ok((config_fields, default_fields))
}

// ### MODIFIED FUNCTION ###
// This function now parses `#[framework::config]`
// and `#[framework::config(default = ...)]`
// It returns an Err if the format is invalid, e.g., `#[framework::config(1000)]`
fn parse_config_field(field: &Field) -> Result<Option<ConfigField>, syn::Error> {
    let Some(field_name) = &field.ident else {
        return Ok(None);
    };

    for attr in &field.attrs {
        let path_str = attr.into_token_stream().to_string().replace(" ", "");
        let is_config = path_str.contains("framework::config");

        if is_config {
            let default_value = match &attr.meta {
                // Case 1: #[framework::config]
                syn::Meta::Path(_) => None,

                // Case 2: #[framework::config(...)]
                syn::Meta::List(meta_list) => {
                    if meta_list.tokens.is_empty() {
                        // Case 2a: #[framework::config()]
                        None
                    } else {
                        // Attempt to parse the tokens as `Meta`
                        match syn::parse2::<syn::Meta>(meta_list.tokens.clone()) {
                            // Case 2b: #[framework::config(default = ...)]
                            Ok(syn::Meta::NameValue(nv)) => {
                                if nv.path.is_ident("default") {
                                    Some(nv.value)
                                } else {
                                    // Case 2c: #[framework::config(other = ...)] - Invalid
                                    return Err(syn::Error::new_spanned(
                                        nv.path,
                                        "Invalid config attribute. Expected `default = ...`",
                                    ));
                                }
                            }
                            // Case 2d: #[framework::config(foo)] or other invalid Meta
                            Ok(other_meta) => {
                                return Err(syn::Error::new_spanned(
                                    other_meta,
                                    "Invalid config attribute format. Expected `#[framework::config(default = ...)]` or `#[framework::config]`"
                                ));
                            }
                            // Case 2e: #[framework::config(1000)] or other invalid syntax
                            Err(_) => {
                                return Err(syn::Error::new_spanned(
                                    meta_list.tokens.clone(),
                                    "Invalid config attribute format. Expected `default = ...`",
                                ));
                            }
                        }
                    }
                }

                // Case 3: #[framework::config = ...] - Invalid
                syn::Meta::NameValue(nv) => {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "Invalid config attribute format. Use `#[framework::config]` or `#[framework::config(default = ...)]`"
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
// ### END MODIFIED FUNCTION ###

fn generate_config_struct(alg_name: &syn::Ident, config_fields: &[ConfigField]) -> TokenStream2 {
    let config_name = syn::Ident::new(
        &format!("{}Config", alg_name),
        proc_macro2::Span::call_site(),
    );

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
            #(#field_defs),*
        }
    }
}

fn generate_peer_methods(handlers: &[HandlerInfo]) -> Vec<TokenStream2> {
    handlers
        .iter()
        .map(|handler| {
            let method_name = &handler.method_name;
            let msg_type = &handler.msg_type;
            let msg_type_str = quote!(#msg_type).to_string().replace(" ", "");

            match &handler.reply_type {
                None => {
                    quote! {
                        pub async fn #method_name(&self, msg: &#msg_type) -> Result<(), ::framework::PeerError> {
                            use ::framework::transport::Connection;
                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_str.to_string(),
                                ::serde_json::to_vec(msg)
                                    .map_err(|e| ::framework::PeerError::SerializationFailed {
                                        message: format!("Failed to serialize message of type '{}': {}", #msg_type_str, e)
                                    })?
                            );

                            let envelope_bytes = ::serde_json::to_vec(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            self.connection.cast(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to cast message: {}", e)
                                })?;

                            Ok(())
                        }
                    }
                }
                Some(reply_type) => {
                    let reply_type_str = quote!(#reply_type).to_string().replace(" ", "");
                    quote! {
                        pub async fn #method_name(&self, msg: &#msg_type) -> Result<#reply_type, ::framework::PeerError> {
                            use ::framework::transport::Connection;
                            let envelope = ::framework::NodeMessage::Algorithm(
                                #msg_type_str.to_string(),
                                ::serde_json::to_vec(msg)
                                    .map_err(|e| ::framework::PeerError::SerializationFailed {
                                        message: format!("Failed to serialize message of type '{}': {}", #msg_type_str, e)
                                    })?
                            );

                            let envelope_bytes = ::serde_json::to_vec(&envelope)
                                .map_err(|e| ::framework::PeerError::SerializationFailed {
                                    message: format!("Failed to serialize envelope: {}", e)
                                })?;

                            let reply_bytes = self.connection.send(envelope_bytes).await
                                .map_err(|e| ::framework::PeerError::TransportError {
                                    message: format!("Failed to send message: {}", e)
                                })?;

                            let reply: #reply_type = ::serde_json::from_slice(&reply_bytes)
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

// ### MODIFIED FUNCTION ###
// Now handles the Result from extract_fields
fn algorithm_setup_impl(item: TokenStream) -> TokenStream {
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
                peers: ::std::collections::HashMap<::framework::community::PeerId, #peer_name<T>>
            };
            fields.named.push(peers_field);
        }
    }

    let field_inits: Vec<_> = config_fields
        .iter()
        .map(|cf| {
            let name = &cf.field_name;
            let default = &cf.default_value;
            if let Some(default) = default {
                quote! { #name: self.#name.unwrap_or(#default) }
            } else {
                let name_str = name.to_string();
                quote! { #name: self.#name.ok_or(::framework::ConfigError::RequiredField { field: #name_str })? }
            }
        })
        .chain(default_fields.iter().map(|f| {
            let name = &f.ident;
            quote! { #name: Default::default() }
        }))
        .collect();

    let initializer_impl = quote! {
        impl<T: ::framework::transport::Transport> ::framework::AlgorithmBuilder<T> for #config_name {
            type Algorithm = #alg_name<T>;
            type Peer = #peer_name<T>;

            fn build(self, peers: ::std::collections::HashMap<::framework::community::PeerId, Self::Peer>) -> Result<::std::sync::Arc<Self::Algorithm>, ::framework::ConfigError> {
                Ok(::std::sync::Arc::new(Self::Algorithm {
                    #(#field_inits,)*
                    peers,
                }))
            }
        }
    };

    // Generate the Peer struct (methods will be added by algorithm_handlers)
    let peer_struct = quote! {
        #[derive(Clone)]
        pub struct #peer_name<T: ::framework::transport::Transport> {
            connection: T::Connection,
        }

        impl<T: ::framework::transport::Transport> #peer_name<T> {
            pub fn new(connection: T::Connection) -> Self {
                Self { connection }
            }
        }
    };

    let expanded = quote! {
        #input

        #config_struct

        #initializer_impl

        #peer_struct
    };

    TokenStream::from(expanded)
}

fn extract_all_handlers(input: &ItemImpl) -> Vec<HandlerInfo> {
    input
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                return parse_handler_method(method);
            }
            None
        })
        .collect()
}

// ### MODIFIED FUNCTION ###
fn algorithm_handlers_impl(item: TokenStream) -> TokenStream {
    let input = match syn::parse::<ItemImpl>(item.clone()) {
        Ok(input) => input,
        Err(_) => {
            return TokenStream::from(quote! {
                compile_error!("Failed to parse item: {}", item);
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

    // Updated to call the renamed function
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

fn message_impl(item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);

    let new_attr_tokens = syn::parse_quote!(#[derive(Serialize, Deserialize, Clone, Debug)]);
    input.attrs.insert(0, new_attr_tokens);

    let output = quote! {
        #input
    };

    output.into()
}

// Export under algorithm:: namespace via naming convention
#[proc_macro_attribute]
pub fn setup(_attr: TokenStream, item: TokenStream) -> TokenStream {
    algorithm_setup_impl(item)
}

#[proc_macro_attribute]
pub fn handlers(_attr: TokenStream, item: TokenStream) -> TokenStream {
    algorithm_handlers_impl(item)
}

#[proc_macro_attribute]
pub fn message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    message_impl(item)
}
