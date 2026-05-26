//! Cross-service gRPC errors for Rust.
//!
//! `aerro` gives every error a **typed identity**, a **bounded call trace**, and
//! a **structured wire encoding**. Derive [`Aerro`] on an error enum, encode it
//! into a `tonic::Status` via `From`/`Into`, and recover the original variant on
//! the client side with `TryFrom`/`TryInto` — with the full chain of service
//! hops attached.
//!
//! # Quick Example
//!
//! ```rust,ignore
//! use aerro::{Aerro, ServiceFailure};
//!
//! #[derive(Debug, aerro::Aerro)]
//! pub enum CreateUserError {
//!     #[aerro(code = Business::AlreadyExists, error = "email already taken: {email}")]
//!     EmailTaken { email: String },
//!
//!     #[aerro(code = System::Internal)]
//!     DbUnavailable,
//! }
//!
//! // Server side
//! let err = CreateUserError::EmailTaken { email: "alice@example.com".into() };
//! let status: tonic::Status = ServiceFailure::from(err).into();
//!
//! // Client side — recover the typed variant
//! let recovered = ServiceFailure::<CreateUserError>::try_from(status).unwrap();
//! ```
//!
//! # Feature Flags
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `macro` | ✓ | [`Aerro`] derive macro |
//! | `tracing` | ✗ | Capture OTel trace/span IDs via the `tracing` subscriber |
//! # Key Types
//!
//! - [`Aerro`] — the trait every error type implements (derive or manual)
//! - [`ServiceFailure<E>`](crate::failure::ServiceFailure) — typed error + frames + trace
//! - [`RemoteError`] — type-erased fallback for errors from unknown services
//! - [`Frame`] — one hop in the call chain
//! - [`TraceContext`] — OTel trace/span IDs
//! - [`EncodeOptions`] — egress configuration (exposure tier, frame cap)

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod any;
pub mod category;
pub mod error;
pub mod exposure;
pub mod failure;
pub mod frame;
pub mod remote;
pub mod trace;
pub mod traits;

#[cfg(test)]
pub(crate) mod test_support;

pub use any::render_chain;
pub use category::Category;
pub use error::{DecodeError, EncodeError};
pub use exposure::Exposure;
pub use failure::ServiceFailure;
pub use frame::Frame;
pub use remote::RemoteError;
pub use trace::TraceContext;
pub use traits::Aerro;

#[cfg(feature = "macro")]
pub use aerro_macros::Aerro;

pub mod convert;
pub mod ext;
pub mod wire;

pub use convert::AerroEncode;
pub use traits::FromServiceFailure;
pub use wire::decode::decode;
pub use wire::encode::{EncodeOptions, encode};

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_version() {
        assert_eq!(crate::VERSION, env!("CARGO_PKG_VERSION"));
    }
}
