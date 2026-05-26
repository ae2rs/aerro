//! Roundtrip integration test: typed error → wire → typed error, exercising
//! all four categories and all three exposure tiers.

#![cfg(feature = "macro")]

use aerro::wire::encode::EncodeOptions;
use aerro::{Aerro, AerroEncode, Category, Exposure, ServiceFailure};
use tonic::Code;

#[derive(Debug, aerro::Aerro)]
pub enum Suite {
    #[aerro(code = Business::AlreadyExists, error = "biz: {0}")]
    Biz(String),

    #[aerro(code = Validation::InvalidArgument, error = "val: {0}")]
    Val(String),

    #[aerro(code = System::Internal, error = "sys")]
    Sys,

    #[aerro(code = Transport::Unavailable, error = "trans")]
    Trans,
}

fn opts(exposure: Exposure) -> EncodeOptions {
    EncodeOptions {
        exposure,
        max_frames: 16,
    }
}

#[test]
fn business_internal_roundtrips_payload() {
    let st = Suite::Biz("dup".into()).encode_with_opts(&opts(Exposure::Internal));
    let sf = ServiceFailure::<Suite>::try_from(st).unwrap();
    match sf.into_inner() {
        Suite::Biz(s) => assert_eq!(s, "dup"),
        _ => panic!(),
    }
}

#[test]
fn validation_public_roundtrips_payload_and_keeps_message() {
    let st = Suite::Val("bad".into()).encode_with_opts(&opts(Exposure::Public));
    assert_eq!(st.code(), Code::InvalidArgument);
    assert_eq!(st.message(), "val: bad"); // Validation is safe at Public.
    let sf = ServiceFailure::<Suite>::try_from(st).unwrap();
    assert!(matches!(sf.inner(), Suite::Val(_)));
}

#[test]
fn system_public_redacts_message_but_payload_still_decodes() {
    let st = Suite::Sys.encode_with_opts(&opts(Exposure::Public));
    assert_eq!(st.code(), Code::Internal);
    assert_eq!(st.message(), "internal error");
    // The envelope still carries the type_id and decodes typed:
    let sf = ServiceFailure::<Suite>::try_from(st).unwrap();
    assert_eq!(sf.inner().category(), Category::System);
}

#[test]
fn transport_trusted_keeps_message() {
    let st = Suite::Trans.encode_with_opts(&opts(Exposure::Trusted));
    assert_eq!(st.code(), Code::Unavailable);
    assert_eq!(st.message(), "trans");
}

#[test]
fn public_drops_frames_internal_keeps_them() {
    use aerro::Frame;
    let mut sf: ServiceFailure<Suite> = Suite::Sys.into();
    sf.frames_mut().push(Frame::local(
        "svc",
        "rpc",
        Code::Internal,
        "m",
        Category::System,
    ));

    let st_pub = sf.clone_for_test().encode_with_opts(&opts(Exposure::Public));
    let pub_decoded = ServiceFailure::<Suite>::try_from(st_pub).unwrap();
    assert!(pub_decoded.frames().is_empty(), "Public must drop frames");

    let st_int = sf.encode_with_opts(&opts(Exposure::Internal));
    let int_decoded = ServiceFailure::<Suite>::try_from(st_int).unwrap();
    assert_eq!(int_decoded.frames().len(), 1, "Internal must keep frames");
}

// Helper because ServiceFailure isn't Clone; reconstruct manually.
trait CloneForTest {
    fn clone_for_test(&self) -> Self;
}
impl CloneForTest for aerro::ServiceFailure<Suite> {
    fn clone_for_test(&self) -> Self {
        let mut clone: aerro::ServiceFailure<Suite> = match self.inner() {
            Suite::Biz(s) => Suite::Biz(s.clone()),
            Suite::Val(s) => Suite::Val(s.clone()),
            Suite::Sys => Suite::Sys,
            Suite::Trans => Suite::Trans,
        }
        .into();
        for f in self.frames() {
            clone.frames_mut().push(f.clone());
        }
        *clone.trace_mut() = *self.trace();
        clone
    }
}
