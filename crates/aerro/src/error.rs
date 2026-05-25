//! Decode/encode error types — see spec §5.

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("envelope missing")]
    Missing,
    #[error("envelope version {0} unsupported")]
    UnsupportedVersion(u32),
    #[error("unknown type_id `{0}`")]
    UnknownTypeId(String),
    #[error("bincode decode: {0}")]
    Payload(String),
    #[error("prost decode: {0}")]
    Prost(String),
}

#[derive(Debug, thiserror::Error)]
#[error("encode error: {0}")]
pub struct EncodeError(pub String);
