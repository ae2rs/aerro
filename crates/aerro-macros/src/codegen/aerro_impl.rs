//! Emit `impl Aerro for Enum`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::attrs::{EnumCfg, FieldRole, VariantCfg, snake_of_pascal};

pub fn emit_aerro_impl(cfg: &EnumCfg) -> TokenStream {
    let enum_ident = &cfg.ident;
    let enum_snake = snake_of_pascal(&enum_ident.to_string());

    let type_ids: Vec<String> = cfg
        .variants
        .iter()
        .map(|v| format!("{}.{}", enum_snake, snake_of_pascal(&v.ident.to_string())))
        .collect();

    let type_id_arms = cfg.variants.iter().zip(type_ids.iter()).map(|(v, id)| {
        let variant = &v.ident;
        let pat = wild_pat(v);
        quote! { Self::#variant #pat => #id, }
    });

    let category_arms = cfg.variants.iter().map(|v| {
        let variant = &v.ident;
        let pat = wild_pat(v);
        let cat = v.category.ident();
        quote! { Self::#variant #pat => ::aerro::Category::#cat, }
    });

    let code_arms = cfg.variants.iter().map(|v| {
        let variant = &v.ident;
        let pat = wild_pat(v);
        let code = &v.code;
        quote! { Self::#variant #pat => ::tonic::Code::#code, }
    });

    let exposure_arms = cfg.variants.iter().map(|v| {
        let variant = &v.ident;
        let pat = wild_pat(v);
        match v.exposure {
            Some(e) => {
                let e_ident = e.ident();
                quote! { Self::#variant #pat => ::aerro::Exposure::#e_ident, }
            }
            None => {
                let cat = v.category.ident();
                quote! { Self::#variant #pat => ::aerro::Category::#cat.default_exposure(), }
            }
        }
    });

    let encode_arms = cfg.variants.iter().map(encode_payload_arm);
    let decode_arms = cfg
        .variants
        .iter()
        .zip(type_ids.iter())
        .map(|(v, id)| decode_payload_arm(v, id));

    let type_id_lits = type_ids.iter().map(|s| quote! { #s });

    quote! {
        impl ::aerro::Aerro for #enum_ident {
            const TYPE_IDS: &'static [&'static str] = &[ #(#type_id_lits),* ];

            fn type_id(&self) -> &'static str {
                match self {
                    #(#type_id_arms)*
                }
            }

            fn category(&self) -> ::aerro::Category {
                match self {
                    #(#category_arms)*
                }
            }

            fn code(&self) -> ::tonic::Code {
                match self {
                    #(#code_arms)*
                }
            }

            fn exposure(&self) -> ::aerro::Exposure {
                match self {
                    #(#exposure_arms)*
                }
            }

            fn encode_payload(&self, __route: ::aerro::Exposure, __buf: &mut ::std::vec::Vec<u8>) -> ::core::result::Result<(), ::aerro::EncodeError> {
                let _ = __route;
                match self {
                    #(#encode_arms)*
                }
            }

            fn decode_payload(__type_id: &str, __bytes: &[u8]) -> ::core::result::Result<Self, ::aerro::DecodeError> {
                match __type_id {
                    #(#decode_arms)*
                    other => ::core::result::Result::Err(::aerro::DecodeError::UnknownTypeId(other.into())),
                }
            }
        }
    }
}

/// Variant-side wildcard pattern that ignores all fields.
fn wild_pat(v: &VariantCfg) -> TokenStream {
    if v.fields.is_empty() {
        TokenStream::new()
    } else if v.is_tuple {
        let wilds = v.fields.iter().map(|_| quote! { _ });
        quote! { ( #(#wilds),* ) }
    } else {
        quote! { { .. } }
    }
}

/// Bincode payload of bincode-encodable fields only. `#[source]` and `#[from]`
/// fields hold non-bincode error types and are skipped — decode returns
/// `DecodeError::Payload` for these variants, falling back to `RemoteError`.
fn encode_payload_arm(v: &VariantCfg) -> TokenStream {
    let variant = &v.ident;
    let payload_fields: Vec<(usize, &crate::attrs::FieldCfg)> = v
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.role, FieldRole::Plain))
        .collect();

    if v.fields.is_empty() {
        return quote! {
            Self::#variant => {
                ::core::result::Result::Ok(())
            }
        };
    }

    if v.is_tuple {
        // Bind every position. Use _ for non-plain fields.
        let pat_idents: Vec<Ident> = v
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| match f.role {
                FieldRole::Plain => format_ident!("__f{}", i),
                _ => format_ident!("_"),
            })
            .collect();
        let pat = quote! { ( #(#pat_idents),* ) };
        let payload_exprs = payload_fields.iter().map(|(i, f)| {
            let id = format_ident!("__f{}", i);
            redact_expr(f, &quote! { #id })
        });
        quote! {
            Self::#variant #pat => {
                let __tup = ( #(#payload_exprs ,)* );
                let __bytes = ::bincode::encode_to_vec(&__tup, ::bincode::config::standard())
                    .map_err(|e| ::aerro::EncodeError(e.to_string()))?;
                __buf.extend_from_slice(&__bytes);
                ::core::result::Result::Ok(())
            }
        }
    } else {
        // Named-field variant — bind only plain fields; ignore the rest with `..`.
        let names: Vec<&Ident> = payload_fields
            .iter()
            .filter_map(|(_, f)| f.ident.as_ref())
            .collect();
        let pat = quote! { { #(#names,)* .. } };
        let payload_exprs = payload_fields.iter().map(|(_, f)| {
            let name = f.ident.as_ref().unwrap();
            redact_expr(f, &quote! { #name })
        });
        quote! {
            Self::#variant #pat => {
                let __tup = ( #(#payload_exprs ,)* );
                let __bytes = ::bincode::encode_to_vec(&__tup, ::bincode::config::standard())
                    .map_err(|e| ::aerro::EncodeError(e.to_string()))?;
                __buf.extend_from_slice(&__bytes);
                ::core::result::Result::Ok(())
            }
        }
    }
}

fn redact_expr(f: &crate::attrs::FieldCfg, ident_expr: &TokenStream) -> TokenStream {
    let ty = &f.ty;
    if f.redact {
        quote! {
            (if __route == ::aerro::Exposure::Internal {
                ::core::clone::Clone::clone(#ident_expr)
            } else {
                <#ty as ::core::default::Default>::default()
            })
        }
    } else {
        quote! { ::core::clone::Clone::clone(#ident_expr) }
    }
}

fn decode_payload_arm(v: &VariantCfg, type_id: &str) -> TokenStream {
    let variant = &v.ident;
    let plain_fields: Vec<&crate::attrs::FieldCfg> = v
        .fields
        .iter()
        .filter(|f| matches!(f.role, FieldRole::Plain))
        .collect();
    let has_source_or_from = v
        .fields
        .iter()
        .any(|f| matches!(f.role, FieldRole::Source | FieldRole::From));

    if has_source_or_from {
        // Cannot reconstruct anyhow/eyre error from wire — fall back to RemoteError.
        return quote! {
            #type_id => {
                ::core::result::Result::Err(::aerro::DecodeError::Payload(
                    ::std::format!("{} carries an opaque source — decode falls back to RemoteError", #type_id),
                ))
            }
        };
    }

    if v.fields.is_empty() {
        return quote! {
            #type_id => ::core::result::Result::Ok(Self::#variant),
        };
    }

    // Build a tuple type matching the bincode encode side.
    let tup_tys = plain_fields.iter().map(|f| {
        let ty = &f.ty;
        quote! { #ty }
    });

    if v.is_tuple {
        let bindings: Vec<Ident> = plain_fields
            .iter()
            .enumerate()
            .map(|(i, _)| format_ident!("__d{}", i))
            .collect();
        // Reconstruct the full tuple, with Default for non-plain fields.
        let ctor_parts: Vec<TokenStream> = v
            .fields
            .iter()
            .enumerate()
            .scan(0usize, |plain_i, (_, f)| {
                if matches!(f.role, FieldRole::Plain) {
                    let id = &bindings[*plain_i];
                    *plain_i += 1;
                    Some(quote! { #id })
                } else {
                    let ty = &f.ty;
                    Some(quote! { <#ty as ::core::default::Default>::default() })
                }
            })
            .collect();
        let ctor = quote! { Self::#variant ( #(#ctor_parts),* ) };
        quote! {
            #type_id => {
                let (( #(#bindings),* ,), _): (( #(#tup_tys ,)* ), usize) =
                    ::bincode::decode_from_slice(__bytes, ::bincode::config::standard())
                        .map_err(|e| ::aerro::DecodeError::Payload(e.to_string()))?;
                ::core::result::Result::Ok(#ctor)
            }
        }
    } else {
        let names: Vec<&Ident> = plain_fields
            .iter()
            .filter_map(|f| f.ident.as_ref())
            .collect();
        let bindings: Vec<Ident> = names
            .iter()
            .map(|_| format_ident!("__d"))
            .enumerate()
            .map(|(i, _)| format_ident!("__d{}", i))
            .collect();
        let field_parts: Vec<TokenStream> = v
            .fields
            .iter()
            .scan(0usize, |plain_i, f| {
                let name = f.ident.as_ref().unwrap();
                if matches!(f.role, FieldRole::Plain) {
                    let id = &bindings[*plain_i];
                    *plain_i += 1;
                    Some(quote! { #name: #id })
                } else {
                    let ty = &f.ty;
                    Some(quote! { #name: <#ty as ::core::default::Default>::default() })
                }
            })
            .collect();
        let ctor = quote! { Self::#variant { #(#field_parts),* } };
        quote! {
            #type_id => {
                let (( #(#bindings),* ,), _): (( #(#tup_tys ,)* ), usize) =
                    ::bincode::decode_from_slice(__bytes, ::bincode::config::standard())
                        .map_err(|e| ::aerro::DecodeError::Payload(e.to_string()))?;
                ::core::result::Result::Ok(#ctor)
            }
        }
    }
}
