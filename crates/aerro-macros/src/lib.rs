//! Procedural macros for `aerro`. See [`aerro`](https://docs.rs/aerro) for
//! the full usage guide.

mod attrs;
mod codegen;
mod operation;

/// Derive the [`aerro::Aerro`](https://docs.rs/aerro/latest/aerro/trait.Aerro.html)
/// trait for an error enum.
///
/// # Attributes on each variant — `#[aerro(...)]`
///
/// | Key | Type | Required | Description |
/// |-----|------|----------|-------------|
/// | `category` | ident | ✓ | Error bucket: `Business`, `System`, `Auth`, `NotFound`, `RateLimit`, `Validation`, `Unavailable`, `Unknown` |
/// | `code` | ident | ✓ | gRPC status code: `Ok`, `Cancelled`, `Unknown`, `InvalidArgument`, `NotFound`, `AlreadyExists`, `PermissionDenied`, `ResourceExhausted`, `FailedPrecondition`, `Aborted`, `OutOfRange`, `Unimplemented`, `Internal`, `Unavailable`, `DataLoss`, `Unauthenticated` |
/// | `error` | string literal | ✓ | Display format; `{field_name}` interpolates variant fields |
/// | `exposure` | ident | — | Override default exposure: `Internal`, `Trusted`, `Public` (default derived from `category`) |
///
/// # Field-level attributes
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `#[source]` | Marks this field as the `std::error::Error::source()` |
/// | `#[from]` | Generates a `From<FieldType>` impl (field must be the only field) |
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Debug, aerro::Aerro)]
/// pub enum CreateUserError {
///     #[aerro(category = Business, code = AlreadyExists, error = "email already taken: {email}")]
///     EmailTaken { email: String },
///
///     #[aerro(category = System, code = Internal, error = "db.unavailable")]
///     DbUnavailable,
/// }
/// ```
#[proc_macro_derive(Aerro, attributes(aerro, source, from))]
pub fn aerro_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    operation::expand(item.into()).into()
}
