//! Integration tests for `#[aerro(forward)]` field attribute.
//! Verifies that `.forward()` transfers frames from the inner ServiceFailure to the outer one.

#![cfg(feature = "macro")]

use aerro::{Category, Frame, ServiceFailure};
use tonic::Code;

#[derive(Debug, aerro::Aerro)]
enum Inner {
    #[aerro(category = System, code = Internal, error = "inner.fail")]
    Fail,
}

#[derive(Debug, aerro::Aerro)]
enum Outer {
    #[aerro(category = System, code = Internal, error = "outer.wrapped")]
    Wrapped(#[aerro(forward)] Inner),
}

#[test]
fn forward_transfers_frames() {
    let mut sf_inner: ServiceFailure<Inner> = Inner::Fail.into();
    sf_inner.frames_mut().push(Frame::local(
        "svc-a",
        "rpc-a",
        Code::Internal,
        "inner msg",
        Category::System,
    ));

    let sf_outer: ServiceFailure<Outer> = sf_inner.forward();

    assert_eq!(sf_outer.frames().len(), 1);
    assert_eq!(sf_outer.frames()[0].service, "svc-a");
    assert_eq!(sf_outer.frames()[0].message, "inner msg");
    assert!(matches!(sf_outer.inner(), Outer::Wrapped(_)));
}

#[test]
fn empty_frames_transfer_cleanly() {
    let sf_inner: ServiceFailure<Inner> = Inner::Fail.into();
    let sf_outer: ServiceFailure<Outer> = sf_inner.forward();
    assert!(sf_outer.frames().is_empty());
}

#[test]
fn multiple_frames_all_transfer() {
    let mut sf_inner: ServiceFailure<Inner> = Inner::Fail.into();
    sf_inner.frames_mut().push(Frame::local(
        "svc-a", "rpc-a", Code::Internal, "first", Category::System,
    ));
    sf_inner.frames_mut().push(Frame::local(
        "svc-b", "rpc-b", Code::Internal, "second", Category::System,
    ));

    let sf_outer: ServiceFailure<Outer> = sf_inner.forward();

    assert_eq!(sf_outer.frames().len(), 2);
    assert_eq!(sf_outer.frames()[0].service, "svc-a");
    assert_eq!(sf_outer.frames()[1].service, "svc-b");
}

#[test]
fn source_chain_is_accessible() {
    use std::error::Error;
    let sf: ServiceFailure<Outer> = ServiceFailure::new(Outer::Wrapped(Inner::Fail));
    let src = sf.inner().source();
    assert!(src.is_some());
}
