//! Cross-service gRPC errors for Rust.
//!
//! `aerro` is an early-stage crate. The public API is intentionally small
//! while the error model is being designed.

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "macro")]
pub use aerro_macros::{handler, operation};

#[cfg(feature = "tonic")]
pub mod wire {
    //! Wire envelope and prost-generated types.
    pub mod raw {
        include!(concat!(env!("OUT_DIR"), "/aerro.v1.rs"));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_version() {
        assert_eq!(crate::VERSION, env!("CARGO_PKG_VERSION"));
    }
}
