//! Attribute parsing for `#[aerro::operation]`.

use proc_macro2::Span;
use quote::ToTokens;
use syn::{Attribute, DataEnum, Expr, ExprLit, Field, Fields, Ident, Lit, Meta, Token, Variant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryAttr {
    Business,
    System,
    Validation,
    Transport,
}

impl CategoryAttr {
    fn from_str(s: &str, span: Span) -> syn::Result<Self> {
        match s {
            "business" => Ok(Self::Business),
            "system" => Ok(Self::System),
            "validation" => Ok(Self::Validation),
            "transport" => Ok(Self::Transport),
            other => Err(syn::Error::new(
                span,
                format!(
                    "unknown category `{}` (expected one of: business, system, validation, transport)",
                    other
                ),
            )),
        }
    }

    pub fn ident(self) -> Ident {
        match self {
            Self::Business => Ident::new("Business", Span::call_site()),
            Self::System => Ident::new("System", Span::call_site()),
            Self::Validation => Ident::new("Validation", Span::call_site()),
            Self::Transport => Ident::new("Transport", Span::call_site()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposureAttr {
    Internal,
    Trusted,
    Public,
}

impl ExposureAttr {
    fn from_str(s: &str, span: Span) -> syn::Result<Self> {
        match s {
            "internal" => Ok(Self::Internal),
            "trusted" => Ok(Self::Trusted),
            "public" => Ok(Self::Public),
            other => Err(syn::Error::new(
                span,
                format!(
                    "unknown exposure `{}` (expected one of: internal, trusted, public)",
                    other
                ),
            )),
        }
    }

    pub fn ident(self) -> Ident {
        match self {
            Self::Internal => Ident::new("Internal", Span::call_site()),
            Self::Trusted => Ident::new("Trusted", Span::call_site()),
            Self::Public => Ident::new("Public", Span::call_site()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRole {
    Plain,
    Source,
    From,
}

#[derive(Debug, Clone)]
pub struct FieldCfg {
    pub role: FieldRole,
    pub redact: bool,
    pub ident: Option<Ident>,
    pub ty: syn::Type,
}

#[derive(Debug, Clone)]
pub struct VariantCfg {
    pub ident: Ident,
    pub category: CategoryAttr,
    pub code: Ident,
    pub exposure: Option<ExposureAttr>,
    pub error_fmt: Option<String>,
    pub from_catchall: bool,
    pub is_tuple: bool,
    pub fields: Vec<FieldCfg>,
}

#[derive(Debug, Clone)]
pub struct EnumCfg {
    pub ident: Ident,
    pub variants: Vec<VariantCfg>,
}

/// Convert a snake-cased gRPC code (`"already_exists"`) into the matching
/// `tonic::Code::*` ident (`AlreadyExists`).
fn snake_to_pascal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper = true;
    for ch in s.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn snake_of_pascal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i != 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

fn lit_str_of(expr: &Expr) -> Option<&str> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(s), ..
    }) = expr
    {
        Some(s.value().leak()) // leak for stable &str in this macro pass — fine for compile-time use
    } else {
        None
    }
}

fn parse_variant_attr(attr: &Attribute) -> syn::Result<(Option<String>, Option<String>, Option<String>, Option<String>, bool)> {
    // returns (category, code, exposure, error, from_catchall)
    let mut category = None;
    let mut code = None;
    let mut exposure = None;
    let mut error = None;
    let mut from_catchall = false;

    attr.parse_nested_meta(|meta| {
        let key = meta
            .path
            .get_ident()
            .ok_or_else(|| meta.error("expected an identifier"))?
            .to_string();
        match key.as_str() {
            "category" => {
                let value: syn::LitStr = meta.value()?.parse()?;
                category = Some(value.value());
            }
            "code" => {
                let value: syn::LitStr = meta.value()?.parse()?;
                code = Some(value.value());
            }
            "exposure" => {
                let value: syn::LitStr = meta.value()?.parse()?;
                exposure = Some(value.value());
            }
            "error" => {
                let value: syn::LitStr = meta.value()?.parse()?;
                error = Some(value.value());
            }
            "from" => {
                from_catchall = true;
            }
            other => {
                return Err(meta.error(format!("unknown aerro attribute `{}`", other)));
            }
        }
        Ok(())
    })?;

    Ok((category, code, exposure, error, from_catchall))
}

fn collect_field_cfg(field: &Field) -> syn::Result<FieldCfg> {
    let mut role = FieldRole::Plain;
    let mut redact = false;

    for attr in &field.attrs {
        if attr.path().is_ident("source") {
            role = FieldRole::Source;
        } else if attr.path().is_ident("from") {
            role = FieldRole::From;
        } else if attr.path().is_ident("aerro") {
            // field-level `#[aerro(redact)]`
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("redact") {
                    redact = true;
                    Ok(())
                } else {
                    Err(meta.error("field-level aerro attribute only supports `redact`"))
                }
            })?;
        }
    }

    Ok(FieldCfg {
        role,
        redact,
        ident: field.ident.clone(),
        ty: field.ty.clone(),
    })
}

pub fn parse_variant(v: &Variant) -> syn::Result<VariantCfg> {
    let aerro_attrs: Vec<&Attribute> = v
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("aerro"))
        .collect();

    if aerro_attrs.is_empty() {
        return Err(syn::Error::new_spanned(
            v,
            format!(
                "variant `{}` is missing `#[aerro(category = \"...\", code = \"...\")]`",
                v.ident
            ),
        ));
    }
    if aerro_attrs.len() > 1 {
        return Err(syn::Error::new_spanned(
            &aerro_attrs[1].path(),
            "multiple `#[aerro(...)]` attributes on the same variant",
        ));
    }
    let (cat_s, code_s, exp_s, err_s, from_catchall) = parse_variant_attr(aerro_attrs[0])?;

    let category = CategoryAttr::from_str(
        &cat_s.ok_or_else(|| syn::Error::new_spanned(v, "missing `category = \"...\"`"))?,
        v.span(),
    )?;
    let code_snake =
        code_s.ok_or_else(|| syn::Error::new_spanned(v, "missing `code = \"...\"`"))?;
    let code_ident = Ident::new(&snake_to_pascal(&code_snake), v.ident.span());

    let exposure = match exp_s {
        Some(s) => Some(ExposureAttr::from_str(&s, v.span())?),
        None => None,
    };
    if category == CategoryAttr::System && exposure == Some(ExposureAttr::Public) {
        return Err(syn::Error::new_spanned(
            v,
            "variants with `category = \"system\"` cannot be `exposure = \"public\"` — server defects must never be public-by-default",
        ));
    }

    let (is_tuple, fields_iter): (bool, Vec<&Field>) = match &v.fields {
        Fields::Named(named) => (false, named.named.iter().collect()),
        Fields::Unnamed(unnamed) => (true, unnamed.unnamed.iter().collect()),
        Fields::Unit => (false, Vec::new()),
    };
    let fields = fields_iter
        .into_iter()
        .map(collect_field_cfg)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(VariantCfg {
        ident: v.ident.clone(),
        category,
        code: code_ident,
        exposure,
        error_fmt: err_s,
        from_catchall,
        is_tuple,
        fields,
    })
}

pub fn parse_enum(name: Ident, data: &DataEnum) -> syn::Result<EnumCfg> {
    let variants = data
        .variants
        .iter()
        .map(parse_variant)
        .collect::<syn::Result<Vec<_>>>()?;

    // type_id uniqueness lint
    let enum_snake = snake_of_pascal(&name.to_string());
    let mut seen = std::collections::HashSet::new();
    for v in &variants {
        let id = format!("{}.{}", enum_snake, snake_of_pascal(&v.ident.to_string()));
        if !seen.insert(id.clone()) {
            return Err(syn::Error::new_spanned(
                &v.ident,
                format!("duplicate type_id `{}`", id),
            ));
        }
    }

    Ok(EnumCfg {
        ident: name,
        variants,
    })
}

// Re-export for codegen use.
pub use syn::spanned::Spanned as _Spanned;

// Keep `Token` / `_` imports alive without clippy noise.
#[allow(dead_code)]
fn _force_use(_t: Token![,], _l: Lit, _m: Meta, _e: Expr) {}

#[allow(dead_code)]
pub(crate) fn _lit_str(e: &Expr) -> Option<&str> {
    lit_str_of(e)
}

#[allow(dead_code)]
pub(crate) fn _tokens(t: impl ToTokens) -> proc_macro2::TokenStream {
    t.into_token_stream()
}
