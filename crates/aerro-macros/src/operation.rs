//! `#[derive(Operation)]` — derive macro that makes an enum a typed gRPC error.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse2};

use crate::attrs::parse_enum;
use crate::codegen::{emit_aerro_impl, emit_display_and_error};

pub fn expand(item: TokenStream) -> TokenStream {
    let input: DeriveInput = match parse2(item) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };
    let data_enum = match &input.data {
        Data::Enum(d) => d,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "#[derive(Operation)] can only be applied to enums",
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

    // Derive macros only append — the original enum stays in place. Helper
    // attributes (aerro, source, from) are stripped automatically because they
    // are declared in the proc_macro_derive signature.
    quote! {
        #display_and_error
        #aerro_impl
    }
}
