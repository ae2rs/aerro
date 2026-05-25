//! Procedural macros for `aerro`. See [`aerro`](https://docs.rs/aerro) for
//! the full usage guide.

mod attrs;
mod codegen;
mod handler_derive;
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
/// ```rust
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

/// Derive the `AerroHandler` metadata trait for a unit struct, and generate a
/// `call_tonic` method that encodes typed errors into `tonic::Status`.
///
/// The struct must also implement
/// [`aerro::Handler`](https://docs.rs/aerro/latest/aerro/handler/trait.Handler.html).
///
/// # Attributes on the struct — `#[aerro(...)]`
///
/// | Key | Type | Required | Description |
/// |-----|------|----------|-------------|
/// | `service` | string literal | ✓ | Service name injected into each call [`Frame`](https://docs.rs/aerro/latest/aerro/frame/struct.Frame.html) |
/// | `rpc` | string literal | ✓ | RPC name injected into each call `Frame` |
/// | `exposure` | ident | ✓ | Egress exposure tier: `Internal`, `Trusted`, or `Public` |
/// | `max_frames` | integer | — | Frame cap for this handler (default: 16) |
///
/// # Generated items
///
/// - Constants: `SERVICE`, `RPC`, `EXPOSURE`, `MAX_FRAMES`
/// - Method: `async fn call_tonic(&self, req: Request) -> Result<Response, tonic::Status>`
///
/// # Example
///
/// ```rust
/// use aerro::{AerroHandler, Handler};
///
/// #[derive(Debug, aerro::Aerro)]
/// pub enum CreateUserError {
///     #[aerro(category = Business, code = AlreadyExists, error = "email already taken: {email}")]
///     EmailTaken { email: String },
/// }
///
/// #[derive(aerro::AerroHandler)]
/// #[aerro(service = "users", rpc = "create_user", exposure = Public, max_frames = 4)]
/// struct CreateUserHandler;
///
/// impl Handler for CreateUserHandler {
///     type Request = String;
///     type Response = String;
///     type Error = CreateUserError;
///
///     async fn handle(&self, email: String) -> Result<String, CreateUserError> {
///         Ok(format!("created {email}"))
///     }
/// }
/// ```
#[proc_macro_derive(AerroHandler, attributes(aerro))]
pub fn aerro_handler_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    handler_derive::expand(item.into()).into()
}
