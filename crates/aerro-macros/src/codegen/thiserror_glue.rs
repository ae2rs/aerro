//! Emit `Display` + `std::error::Error` impls without depending on `thiserror`
//! at the consumer site.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::attrs::{EnumCfg, FieldRole, VariantCfg, snake_of_pascal};

pub fn emit_display_and_error(cfg: &EnumCfg) -> TokenStream {
    let enum_ident = &cfg.ident;
    let display_arms = cfg.variants.iter().map(|v| display_arm(enum_ident, v));
    let source_arms = cfg.variants.iter().map(source_arm);

    let from_impls = cfg.variants.iter().filter_map(|v| from_impl(enum_ident, v));
    let forward_impls = cfg.variants.iter().filter_map(|v| forward_impl(enum_ident, v));

    quote! {
        impl ::core::fmt::Display for #enum_ident {
            fn fmt(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(#display_arms)*
                }
            }
        }

        impl ::std::error::Error for #enum_ident {
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    #(#source_arms)*
                }
            }
        }

        #(#from_impls)*
        #(#forward_impls)*
    }
}

fn display_arm(enum_ident: &Ident, v: &VariantCfg) -> TokenStream {
    let variant = &v.ident;
    let has_explicit_fmt = v.error_fmt.is_some();
    let default_text = format!(
        "{}.{}",
        snake_of_pascal(&enum_ident.to_string()),
        snake_of_pascal(&variant.to_string())
    );
    let fmt_string = v.error_fmt.clone().unwrap_or(default_text);

    if v.fields.is_empty() {
        return quote! {
            Self::#variant => ::core::write!(__f, #fmt_string),
        };
    }

    if v.is_tuple {
        // Bind Plain fields for positional format args; use _ for Source/From/Forward
        // so they don't become stray unused arguments in write!().
        let pat_idents: Vec<Ident> = v
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                if matches!(f.role, FieldRole::Plain) {
                    format_ident!("__f{}", i)
                } else {
                    format_ident!("_")
                }
            })
            .collect();
        let plain_idents: Vec<Ident> = v
            .fields
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if matches!(f.role, FieldRole::Plain) {
                    Some(format_ident!("__f{}", i))
                } else {
                    None
                }
            })
            .collect();
        let pat = quote! { ( #(#pat_idents),* ) };
        if has_explicit_fmt {
            quote! {
                Self::#variant #pat => ::core::write!(__f, #fmt_string, #(#plain_idents),*),
            }
        } else {
            quote! {
                Self::#variant #pat => ::core::write!(__f, #fmt_string),
            }
        }
    } else {
        let names: Vec<&Ident> = v.fields.iter().filter_map(|f| f.ident.as_ref()).collect();
        let pat = quote! { { #(#names),* } };
        if has_explicit_fmt {
            let kwargs = names.iter().map(|n| quote! { #n = #n });
            quote! {
                Self::#variant #pat => ::core::write!(__f, #fmt_string #(, #kwargs)*),
            }
        } else {
            quote! {
                Self::#variant { .. } => ::core::write!(__f, #fmt_string),
            }
        }
    }
}

fn source_arm(v: &VariantCfg) -> TokenStream {
    let variant = &v.ident;
    if v.fields.is_empty() {
        return quote! { Self::#variant => ::core::option::Option::None, };
    }

    let src_idx = v
        .fields
        .iter()
        .position(|f| matches!(f.role, FieldRole::Source | FieldRole::From | FieldRole::Forward));

    if let Some(idx) = src_idx {
        if v.is_tuple {
            let pat_idents: Vec<Ident> = v
                .fields
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    if i == idx {
                        format_ident!("__src")
                    } else {
                        format_ident!("_")
                    }
                })
                .collect();
            quote! {
                Self::#variant ( #(#pat_idents),* ) => ::core::option::Option::Some(
                    __src as &(dyn ::std::error::Error + 'static)
                ),
            }
        } else {
            let mut pat = TokenStream::new();
            for (i, f) in v.fields.iter().enumerate() {
                let name = f.ident.as_ref().unwrap();
                if i == idx {
                    pat.extend(quote! { #name: __src, });
                } else {
                    pat.extend(quote! { #name: _, });
                }
            }
            quote! {
                Self::#variant { #pat } => ::core::option::Option::Some(
                    __src as &(dyn ::std::error::Error + 'static)
                ),
            }
        }
    } else {
        // Plain variants without #[source]/#[from]
        if v.is_tuple {
            let wild = v.fields.iter().map(|_| quote! { _ });
            quote! { Self::#variant ( #(#wild),* ) => ::core::option::Option::None, }
        } else {
            quote! { Self::#variant { .. } => ::core::option::Option::None, }
        }
    }
}

fn from_impl(enum_ident: &Ident, v: &VariantCfg) -> Option<TokenStream> {
    if !v.from_catchall {
        return None;
    }
    let variant = &v.ident;
    let from_field = v.fields.iter().find(|f| f.role == FieldRole::From)?;
    let ty = &from_field.ty;
    // Build the constructor expression.
    let ctor = if v.is_tuple {
        quote! { #enum_ident::#variant(__from) }
    } else if let Some(name) = &from_field.ident {
        quote! { #enum_ident::#variant { #name: __from } }
    } else {
        return None;
    };
    Some(quote! {
        impl ::core::convert::From<#ty> for #enum_ident {
            fn from(__from: #ty) -> Self {
                #ctor
            }
        }
    })
}

fn forward_impl(enum_ident: &Ident, v: &VariantCfg) -> Option<TokenStream> {
    let forward_field = v.fields.iter().find(|f| f.role == FieldRole::Forward)?;
    let variant = &v.ident;
    let ty = &forward_field.ty;

    let ctor = if v.is_tuple {
        quote! { #enum_ident::#variant(__inner) }
    } else if let Some(name) = &forward_field.ident {
        quote! { #enum_ident::#variant { #name: __inner } }
    } else {
        return None;
    };

    // Implement the local trait on the local enum type — avoids the orphan rule
    // that prevents `impl From<ServiceFailure<T>> for ServiceFailure<Outer>` in
    // downstream crates. Use `sf.forward::<Outer>()` to perform the conversion.
    Some(quote! {
        impl ::aerro::FromServiceFailure<#ty> for #enum_ident {
            fn from_failure(__sf: ::aerro::ServiceFailure<#ty>) -> ::aerro::ServiceFailure<Self> {
                let (__inner, __frames, __trace) = __sf.into_parts();
                ::aerro::ServiceFailure::from_parts(#ctor, __frames, __trace)
            }
        }
    })
}

// silence unused imports if the file is parsed standalone
#[allow(dead_code)]
fn _span_anchor() -> Span {
    Span::call_site()
}
