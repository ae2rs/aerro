//! Universal trait every typed error implements — see spec §5.

use tonic::Code;

use crate::{Category, Exposure, error::{DecodeError, EncodeError}};

/// Universal trait implemented by every typed error.
///
/// Manual impls are allowed; the derive in `aerro-macros` emits one per
/// variant of a `#[aerro::operation]` enum.
pub trait Aerro: std::error::Error + Send + Sync + 'static {
    /// Every `type_id` this error type can produce — used by `wire::decode`
    /// to quickly reject mismatched envelopes without attempting bincode.
    const TYPE_IDS: &'static [&'static str];

    /// Stable, version-pinned identifier for the *current* variant
    /// (e.g. `"create_user.email_taken"`).
    fn type_id(&self) -> &'static str;

    /// Variant taxonomy bucket.
    fn category(&self) -> Category;

    /// gRPC code this variant maps to.
    fn code(&self) -> Code;

    /// Exposure declared on this variant (override) or the category default.
    fn exposure(&self) -> Exposure {
        self.category().default_exposure()
    }

    /// Encode the variant's payload into bincode bytes. `route` is the
    /// exposure level of the destination; fields marked `#[aerro(redact)]`
    /// are replaced with `Default::default()` whenever `route != Internal`.
    fn encode_payload(&self, route: Exposure, buf: &mut Vec<u8>) -> Result<(), EncodeError>;

    /// Decode a typed variant from a `type_id` + bincode bytes.
    fn decode_payload(type_id: &str, bytes: &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized;
}
