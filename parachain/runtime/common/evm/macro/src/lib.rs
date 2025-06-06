use proc_macro::TokenStream;
use proc_macro2::{Group, Span, TokenTree};
use quote::quote;
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

fn parse_selector(signature_lit: syn::LitStr) -> u32 {
    let signature = signature_lit.value();
    let digest = Keccak256::digest(signature.as_bytes());
    let selector = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    selector
}

fn parse_call_enum(input: DeriveInput) -> syn::Result<TokenStream> {
    let enum_ident = input.ident.clone();
    let variants = if let syn::Data::Enum(syn::DataEnum { variants, .. }) = input.data {
        variants
    } else {
        return Err(syn::Error::new(input.ident.span(), "Structure not supported"));
    };

    struct Call {
        variant: syn::Variant,
    }

    let mut selector_to_call = BTreeMap::new();

    for v in variants {
        for a in &v.attrs {
            match a.parse_meta() {
                Ok(syn::Meta::NameValue(syn::MetaNameValue {
                    path: syn::Path { segments, .. },
                    lit: syn::Lit::Str(signature_lit),
                    ..
                })) if segments.first().filter(|path| path.ident == "selector").is_some() => {
                    selector_to_call.insert(parse_selector(signature_lit), Call { variant: v.clone() });
                    for f in &v.fields {
                        if f.ident.is_none() {
                            return Err(syn::Error::new(f.span(), "Unnamed fields not supported"));
                        }
                    }
                }
                _ => return Err(syn::Error::new(a.span(), "Attribute not supported")),
            }
        }
    }

    let selectors: Vec<_> = selector_to_call.keys().collect();
    let variants_ident: Vec<_> = selector_to_call
        .values()
        .map(|Call { variant, .. }| variant.ident.clone())
        .collect();
    let variants_args: Vec<Vec<_>> = selector_to_call
        .values()
        .map(|Call { variant, .. }| {
            variant
                .fields
                .iter()
                .map(|field| field.ident.clone().expect("Only named fields supported"))
                .collect()
        })
        .collect();

    Ok(quote! {
        impl #enum_ident {
            pub fn new(input: &[u8]) -> ::evm_utils::EvmResult<Self> {
                use ::evm_utils::RevertReason;

                let mut reader = ::evm_utils::Reader::new(input);
                let selector = reader.read_selector()?;
                match selector {
                    #(
                        #selectors => Ok(Self::#variants_ident {
                            #(
                                #variants_args: reader.read()?
                            ),*
                        }),
                    )*
                    _ => Err(RevertReason::UnknownSelector.into())
                }
            }
        }
    }
    .into())
}

#[proc_macro_derive(EvmCall, attributes(selector))]
pub fn precompile_calls(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse_call_enum(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}

fn parse_event_enum(input: DeriveInput) -> syn::Result<TokenStream> {
    let enum_ident = input.ident.clone();
    let variants = if let syn::Data::Enum(syn::DataEnum { variants, .. }) = input.data {
        variants
    } else {
        return Err(syn::Error::new(input.ident.span(), "Structure not supported"));
    };

    struct Event {
        variant: syn::Variant,
        topics: Vec<TokenTree>,
        // NOTE: we do not yet support tuple encoding
        data: Option<syn::Ident>,
    }

    let mut selector_to_event = BTreeMap::new();

    for v in variants {
        for a in &v.attrs {
            match a.parse_meta() {
                Ok(syn::Meta::NameValue(syn::MetaNameValue {
                    path: syn::Path { segments, .. },
                    lit: syn::Lit::Str(signature_lit),
                    ..
                })) if segments.first().filter(|path| path.ident == "selector").is_some() => {
                    let selector = Keccak256::digest(signature_lit.value().as_bytes()).to_vec();
                    selector_to_event.insert(
                        selector.clone(),
                        Event {
                            variant: v.clone(),
                            topics: vec![TokenTree::Group(Group::new(
                                proc_macro2::Delimiter::Bracket,
                                quote!(#(#selector),*),
                            ))],
                            data: None,
                        },
                    );
                    if let syn::Fields::Named(syn::FieldsNamed { ref named, .. }) = v.fields {
                        for n in named {
                            let param = n
                                .ident
                                .clone()
                                .ok_or(syn::Error::new(n.span(), "Unnamed fields not supported"))?;

                            match n.attrs.first().map(|attr| attr.parse_meta()) {
                                Some(Ok(syn::Meta::Path(syn::Path { segments, .. })))
                                    if segments.first().filter(|path| path.ident == "indexed").is_some() =>
                                {
                                    if let Some(event) = selector_to_event.get_mut(&selector) {
                                        event.topics.push(TokenTree::Ident(param))
                                    }
                                }
                                _ => {
                                    if let Some(event) = selector_to_event.get_mut(&selector) {
                                        if event.data.is_some() {
                                            return Err(syn::Error::new(n.span(), "Only one data field is allowed"));
                                        } else {
                                            event.data = Some(param)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => return Err(syn::Error::new(a.span(), "Attribute not supported")),
            }
        }
    }

    let variants_ident: Vec<_> = selector_to_event
        .values()
        .map(|Event { variant, .. }| variant.ident.clone())
        .collect();
    let variants_args: Vec<Vec<_>> = selector_to_event
        .values()
        .map(|Event { variant, .. }| {
            variant
                .fields
                .iter()
                .map(|arg| arg.ident.as_ref().expect("Named field"))
                .collect()
        })
        .collect();
    let topics: Vec<Vec<_>> = selector_to_event
        .values()
        .map(|Event { topics, .. }| topics.clone())
        .collect();
    let data: Vec<_> = selector_to_event
        .values()
        .map(|Event { data, .. }| {
            data.clone()
                .ok_or(syn::Error::new(Span::call_site(), "Requires data field"))
        })
        .collect::<Result<_, _>>()?;

    Ok(quote! {
        impl #enum_ident {
            pub fn log(self, handle: &mut impl ::fp_evm::PrecompileHandle) -> ::evm_utils::EvmResult {
                let (topics, data): (Vec<::sp_core::H256>, _) = match self {
                    #(
                        Self::#variants_ident { #(#variants_args),* } => {
                            (vec![#(#topics.into()),*], #data)
                        },
                    )*
                };

                let mut writer = ::evm_utils::Writer::new();
                let data = writer.write(data).build();

                handle.record_cost(::evm_utils::log_cost(topics.len(), data.len())?)?;

                handle.log(
                    handle.context().address,
                    topics,
                    data,
                )?;
                Ok(())
            }
        }
    }
    .into())
}

#[proc_macro_derive(EvmEvent, attributes(selector, indexed, data))]
pub fn precompile_events(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse_event_enum(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}
