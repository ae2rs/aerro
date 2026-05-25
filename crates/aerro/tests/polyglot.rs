//! A "polyglot" consumer that doesn't link aerro must still see correct
//! `Status.code()` and `Status.message()`. The aerro envelope in `details()`
//! is opaque to it but the gRPC contract is preserved.

#![cfg(feature = "macro")]

use aerro::IntoStatus;
use aerro::wire::encode::EncodeOptions;
use tonic::Code;

#[derive(Debug, aerro::Aerro)]
pub enum Api {
    #[aerro(category = "business", code = "not_found", error = "user not found")]
    NotFound,

    #[aerro(category = "system", code = "internal", error = "internal crash")]
    Boom,
}

#[test]
fn bare_tonic_consumer_sees_correct_code_and_message_internal() {
    let st = Api::NotFound.into_status(&EncodeOptions {
        exposure: aerro::Exposure::Internal,
        max_frames: 16,
    });
    // A consumer that knows nothing about aerro only inspects code + message.
    assert_eq!(st.code(), Code::NotFound);
    assert_eq!(st.message(), "user not found");
}

#[test]
fn bare_tonic_consumer_sees_redacted_message_at_public_for_system() {
    let st = Api::Boom.into_status(&EncodeOptions {
        exposure: aerro::Exposure::Public,
        max_frames: 16,
    });
    assert_eq!(st.code(), Code::Internal);
    assert_eq!(st.message(), "internal error");
}

#[test]
fn details_bytes_are_additive_not_required() {
    let st = Api::NotFound.into_status(&EncodeOptions::default());
    // The details() carry the aerro envelope, but the consumer is free to
    // ignore them — code + message alone are well-defined.
    assert!(!st.details().is_empty());
}
