//! Wire envelope (prost) + encode/decode glue.

pub mod raw {
    include!(concat!(env!("OUT_DIR"), "/aerro.v1.rs"));
}

pub mod decode;
pub mod encode;
pub mod envelope;
