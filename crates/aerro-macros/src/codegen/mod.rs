//! Code generation for the `#[aerro::operation]` macro.

pub mod aerro_impl;
pub mod thiserror_glue;

pub use aerro_impl::emit_aerro_impl;
pub use thiserror_glue::emit_display_and_error;
