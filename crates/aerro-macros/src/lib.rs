//! Procedural macros for `aerro`. Implementation lives in sibling modules.

mod attrs;
mod codegen;
mod handler;
mod operation;

#[proc_macro_derive(Aerro, attributes(aerro, source, from))]
pub fn aerro_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    operation::expand(item.into()).into()
}

#[proc_macro_attribute]
pub fn handler(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    handler::expand(args.into(), item.into()).into()
}
