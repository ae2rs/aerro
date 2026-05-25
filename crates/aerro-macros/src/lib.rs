//! Procedural macros for `aerro`. Implementation lives in sibling modules.

mod attrs;
mod codegen;
mod handler_derive;
mod operation;

#[proc_macro_derive(Aerro, attributes(aerro, source, from))]
pub fn aerro_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    operation::expand(item.into()).into()
}

#[proc_macro_derive(AerroHandler, attributes(aerro))]
pub fn aerro_handler_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    handler_derive::expand(item.into()).into()
}
