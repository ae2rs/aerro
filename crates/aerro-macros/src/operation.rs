//! `#[aerro::operation]` — annotate an enum to make it a typed gRPC error.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse2};

use crate::attrs::parse_enum;
use crate::codegen::{emit_aerro_impl, emit_display_and_error};

pub fn expand(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input: DeriveInput = match parse2(item.clone()) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };
    let data_enum = match &input.data {
        Data::Enum(d) => d,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "#[aerro::operation] can only be applied to enums",
            )
            .to_compile_error();
        }
    };
    let cfg = match parse_enum(input.ident.clone(), data_enum) {
        Ok(c) => c,
        Err(e) => return e.to_compile_error(),
    };
    let display_and_error = emit_display_and_error(&cfg);
    let aerro_impl = emit_aerro_impl(&cfg);

    // Strip `#[aerro(...)]` attributes from the original enum (and from its
    // variants/fields) so the user-facing tokens stay valid Rust.
    let mut cleaned = strip_aerro_attrs(input);
    // The `Aerro` trait requires `Debug` — ensure the enum has one even if the
    // user didn't write one. We always inject `#[derive(Debug)]` for ergonomics.
    let inject_debug: syn::Attribute = syn::parse_quote!(#[derive(::core::fmt::Debug)]);
    cleaned.attrs.insert(0, inject_debug);

    quote! {
        #cleaned
        #display_and_error
        #aerro_impl
    }
}

fn strip_aerro_attrs(mut input: DeriveInput) -> DeriveInput {
    input.attrs.retain(|a| !a.path().is_ident("aerro"));
    if let Data::Enum(ref mut data_enum) = input.data {
        for v in &mut data_enum.variants {
            v.attrs.retain(|a| !a.path().is_ident("aerro"));
            match &mut v.fields {
                syn::Fields::Named(named) => {
                    for f in &mut named.named {
                        f.attrs.retain(|a| {
                            !a.path().is_ident("aerro")
                                && !a.path().is_ident("source")
                                && !a.path().is_ident("from")
                        });
                    }
                }
                syn::Fields::Unnamed(unnamed) => {
                    for f in &mut unnamed.unnamed {
                        f.attrs.retain(|a| {
                            !a.path().is_ident("aerro")
                                && !a.path().is_ident("source")
                                && !a.path().is_ident("from")
                        });
                    }
                }
                syn::Fields::Unit => {}
            }
        }
    }
    input
}
