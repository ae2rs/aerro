//! Cross-service gRPC errors for Rust.
//!
//! `aerro` gives every error a **typed identity**, a **bounded call trace**, and
//! a **structured wire encoding**. Derive [`Aerro`] on an error enum, call
//! [`IntoStatus`] to encode it into a `tonic::Status`, and [`TryFromStatus`]
//! on the client to recover the original variant — with the full chain of service
//! hops attached.
//!
//! # Quick Example
//!
//! ```rust
//! use aerro::{Aerro, IntoStatus, StatusIntoResultExt};
//! use aerro::wire::encode::EncodeOptions;
//!
//! #[derive(Debug, aerro::Aerro)]
//! pub enum CreateUserError {
//!     #[aerro(category = Business, code = AlreadyExists, error = "email already taken: {email}")]
//!     EmailTaken { email: String },
//!
//!     #[aerro(category = System, code = Internal, error = "db.unavailable")]
//!     DbUnavailable,
//! }
//!
//! // Server side
//! let err = CreateUserError::EmailTaken { email: "alice@example.com".into() };
//! let status = err.into_status(&EncodeOptions::default());
//!
//! // Client side — recover the typed variant
//! let recovered = status.into_aerro::<CreateUserError>().unwrap();
//! ```
//!
//! # Feature Flags
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `macro` | ✓ | [`Aerro`] and [`AerroHandler`] derive macros |
//! | `tracing` | ✓ | Capture OTel trace/span IDs via the `tracing` subscriber |
//! | `anyhow` | — | `AnyError` bridge for `anyhow::Error` |
//! | `eyre` | — | `AnyError` bridge for `eyre::Report` |
//! | `compat-json` | — | JSON wire envelope (alternative to default protobuf) |
//!
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
pub mod handler;
pub mod remote;
pub mod trace;
pub mod traits;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(any(feature = "anyhow", feature = "eyre"))]
pub use any::AnyError;
pub use any::render_chain;
pub use category::Category;
pub use error::{DecodeError, EncodeError};
pub use exposure::Exposure;
pub use failure::ServiceFailure;
pub use frame::Frame;
pub use handler::{AerroHandler, Handler};
pub use remote::RemoteError;
pub use trace::TraceContext;
pub use traits::Aerro;

#[cfg(feature = "macro")]
pub use aerro_macros::{Aerro, AerroHandler};

#[cfg(feature = "compat-json")]
pub mod compat_json;
pub mod ext;
pub mod tower;
pub mod wire;

pub use ext::{ResultIntoStatusExt, StatusIntoResultExt};
pub use traits::{IntoStatus, TryFromStatus};
pub use wire::decode::decode;
pub use wire::encode::{EncodeOptions, encode};

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_version() {
        assert_eq!(crate::VERSION, env!("CARGO_PKG_VERSION"));
    }
}
