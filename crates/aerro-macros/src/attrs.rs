//! Attribute parsing for `#[aerro::operation]`.

use proc_macro2::Span;
use quote::ToTokens;
use syn::{Attribute, DataEnum, Expr, Field, Fields, Ident, Lit, Meta, Token, Variant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryAttr {
    Business,
    System,
    Validation,
    Transport,
}

impl CategoryAttr {
    fn from_ident(ident: &Ident) -> syn::Result<Self> {
        match ident.to_string().as_str() {
            "Business" => Ok(Self::Business),
            "System" => Ok(Self::System),
            "Validation" => Ok(Self::Validation),
            "Transport" => Ok(Self::Transport),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown category `{}` (expected one of: Business, System, Validation, Transport)",
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
    pub(crate) fn from_ident(ident: &Ident) -> syn::Result<Self> {
        match ident.to_string().as_str() {
            "Internal" => Ok(Self::Internal),
            "Trusted" => Ok(Self::Trusted),
            "Public" => Ok(Self::Public),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown exposure `{}` (expected one of: Internal, Trusted, Public)",
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

#[derive(Default)]
struct ParsedVariantAttr {
    category: Option<Ident>,
    code: Option<Ident>,
    exposure: Option<Ident>,
    error: Option<String>,
    from_catchall: bool,
}

fn parse_variant_attr(attr: &Attribute) -> syn::Result<ParsedVariantAttr> {
    let mut out = ParsedVariantAttr::default();

    attr.parse_nested_meta(|meta| {
        let key = meta
            .path
            .get_ident()
            .ok_or_else(|| meta.error("expected an identifier"))?
            .to_string();
        match key.as_str() {
            "category" => {
                let value: Ident = meta.value()?.parse()?;
                out.category = Some(value);
            }
            "code" => {
                let value: Ident = meta.value()?.parse()?;
                out.code = Some(value);
            }
            "exposure" => {
                let value: Ident = meta.value()?.parse()?;
                out.exposure = Some(value);
            }
            "error" => {
                let value: syn::LitStr = meta.value()?.parse()?;
                out.error = Some(value.value());
            }
            "from" => {
                out.from_catchall = true;
            }
            other => {
                return Err(meta.error(format!("unknown aerro attribute `{}`", other)));
            }
        }
        Ok(())
    })?;

    Ok(out)
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

fn validate_error_fmt(
    fmt: &str,
    is_tuple: bool,
    fields: &[FieldCfg],
    span: Span,
) -> syn::Result<()> {
    let field_names: Vec<String> = fields
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .map(|i| i.to_string())
        .collect();

    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '{' {
            if c == '}' {
                chars.next(); // skip escaped }}
            }
            continue;
        }
        if chars.peek() == Some(&'{') {
            chars.next(); // skip escaped {{
            continue;
        }
        // collect placeholder name up to } or :
        let mut name = String::new();
        for inner in chars.by_ref() {
            if inner == '}' || inner == ':' {
                if inner == ':' {
                    // consume rest of format spec
                    for spec in chars.by_ref() {
                        if spec == '}' {
                            break;
                        }
                    }
                }
                break;
            }
            name.push(inner);
        }
        if name.is_empty() || name.parse::<usize>().is_ok() {
            // positional placeholder {} or {0} — skip
            continue;
        }
        if is_tuple {
            return Err(syn::Error::new(
                span,
                format!(
                    "named placeholder `{{{}}}` cannot be used in a tuple variant (use positional `{{}}` or `{{0}}` instead)",
                    name
                ),
            ));
        }
        if !field_names.iter().any(|f| f == &name) {
            let available = if field_names.is_empty() {
                "(none)".to_string()
            } else {
                field_names.join(", ")
            };
            return Err(syn::Error::new(
                span,
                format!(
                    "format placeholder `{{{}}}` does not match any field (available: {})",
                    name, available
                ),
            ));
        }
    }
    Ok(())
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
                "variant `{}` is missing `#[aerro(category = Business, code = AlreadyExists)]`",
                v.ident
            ),
        ));
    }
    if aerro_attrs.len() > 1 {
        return Err(syn::Error::new_spanned(
            aerro_attrs[1].path(),
            "multiple `#[aerro(...)]` attributes on the same variant",
        ));
    }
    let parsed = parse_variant_attr(aerro_attrs[0])?;

    let category_ident = parsed.category.ok_or_else(|| {
        syn::Error::new_spanned(
            v,
            "missing `category = Business` (one of: Business, System, Validation, Transport)",
        )
    })?;
    let category = CategoryAttr::from_ident(&category_ident)?;

    let code_ident = parsed.code.ok_or_else(|| {
        syn::Error::new_spanned(
            v,
            "missing `code = AlreadyExists` (PascalCase tonic::Code variant name)",
        )
    })?;

    let exposure = match parsed.exposure {
        Some(ident) => Some(ExposureAttr::from_ident(&ident)?),
        None => None,
    };
    if category == CategoryAttr::System && exposure == Some(ExposureAttr::Public) {
        return Err(syn::Error::new_spanned(
            v,
            "variants with `category = System` cannot be `exposure = Public` — server defects must never be public-by-default",
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

    if let Some(ref fmt) = parsed.error {
        validate_error_fmt(fmt, is_tuple, &fields, v.span())?;
    }

    Ok(VariantCfg {
        ident: v.ident.clone(),
        category,
        code: code_ident,
        exposure,
        error_fmt: parsed.error,
        from_catchall: parsed.from_catchall,
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
pub(crate) fn _tokens(t: impl ToTokens) -> proc_macro2::TokenStream {
    t.into_token_stream()
}
