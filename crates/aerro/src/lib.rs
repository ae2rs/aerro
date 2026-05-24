//! Cross-service gRPC errors for Rust.
//!
//! `aerro` is the open-source Rust gRPC error library — cross-service typed
//! errors with bounded inline call traces, structured upcasting/downcasting,
//! and zero allocations on the happy path.
//!
//! See `docs/specs/2026-05-24-aerro-v1-design.md` for the full design spec.

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
#[cfg(any(feature = "anyhow", feature = "eyre"))]
pub use any::AnyError;
pub use category::Category;
pub use error::{DecodeError, EncodeError};
pub use exposure::Exposure;
pub use failure::ServiceFailure;
pub use frame::Frame;
pub use remote::RemoteError;
pub use trace::TraceContext;
pub use traits::Aerro;

#[cfg(feature = "macro")]
pub use aerro_macros::{handler, operation};

#[cfg(feature = "compat-json")]
pub mod compat_json;
pub mod ext;
pub mod tower;
pub mod wire;

pub use ext::{ResultIntoStatusExt, StatusIntoResultExt};
pub use traits::{IntoStatus, TryFromStatus};
pub use wire::encode::{EncodeOptions, encode};
pub use wire::decode::decode;

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_version() {
        assert_eq!(crate::VERSION, env!("CARGO_PKG_VERSION"));
    }
}
