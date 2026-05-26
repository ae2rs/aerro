//! Verify the bincode wire envelope round-trips at the byte level.

#![cfg(feature = "macro")]

use aerro::wire::decode::decode;
use aerro::wire::encode::{EncodeOptions, encode};
use aerro::{Aerro, Category, ServiceFailure};
use tonic::Code;

#[derive(Debug, aerro::Aerro)]
pub enum Ping {
    #[aerro(category = Business, code = AlreadyExists, error = "ping")]
    Pong,
}

#[test]
fn envelope_roundtrips_via_bincode() {
    let sf: ServiceFailure<Ping> = Ping::Pong.into();
    let st = encode(&sf, &EncodeOptions::default());
    assert!(!st.details().is_empty(), "envelope must be present");
    let back = decode::<Ping>(st).expect("roundtrip");
    assert_eq!(back.inner().category(), Category::Business);
    assert_eq!(back.inner().code(), Code::AlreadyExists);
}
