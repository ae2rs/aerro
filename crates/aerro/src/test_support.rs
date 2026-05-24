//! Shared test fixtures. Compiled only under `cfg(test)`.

use bincode::{Decode, Encode};
use tonic::Code;

use crate::{Aerro, Category, Exposure, error::DecodeError};

#[derive(Debug, thiserror::Error, Encode, Decode)]
#[error("toy.boom (x={x})")]
pub struct Boom {
    pub x: u32,
}

impl Aerro for Boom {
    const TYPE_IDS: &'static [&'static str] = &["toy.boom"];

    fn type_id(&self) -> &'static str {
        "toy.boom"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn code(&self) -> Code {
        Code::Internal
    }

    fn encode_payload(&self, _route: Exposure, buf: &mut Vec<u8>) {
        let bytes = bincode::encode_to_vec(self, bincode::config::standard()).unwrap();
        buf.extend_from_slice(&bytes);
    }

    fn decode_payload(type_id: &str, bytes: &[u8]) -> Result<Self, DecodeError> {
        if type_id != "toy.boom" {
            return Err(DecodeError::UnknownTypeId(type_id.into()));
        }
        let (v, _) = bincode::decode_from_slice(bytes, bincode::config::standard())
            .map_err(|e| DecodeError::Payload(e.to_string()))?;
        Ok(v)
    }
}
