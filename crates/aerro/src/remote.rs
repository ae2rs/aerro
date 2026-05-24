//! Type-erased fallback for unknown wire types — see spec §5.

use bytes::Bytes;
use smallvec::SmallVec;
use tonic::Code;

use crate::{Aerro, Category, Frame, trace::TraceContext};

#[derive(Debug)]
pub struct RemoteError {
    pub category: Category,
    pub type_id: String,
    pub frames: SmallVec<[Frame; 4]>,
    pub trace: TraceContext,
    pub outer_code: Code,
    pub outer_message: String,
    pub(crate) payload_bytes: Bytes,
}

impl RemoteError {
    /// Recover a typed variant whose `type_id` is in `E::TYPE_IDS`.
    pub fn downcast<E: Aerro>(&self) -> Option<E> {
        if !E::TYPE_IDS.contains(&self.type_id.as_str()) {
            return None;
        }
        E::decode_payload(&self.type_id, &self.payload_bytes).ok()
    }
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.outer_code, self.outer_message)
    }
}

impl std::error::Error for RemoteError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;

    #[test]
    fn downcast_recovers_known_type() {
        let mut buf = Vec::new();
        Boom { x: 7 }.encode_payload(&mut buf);
        let r = RemoteError {
            category: Category::System,
            type_id: "toy.boom".into(),
            frames: SmallVec::new(),
            trace: TraceContext::default(),
            outer_code: Code::Internal,
            outer_message: "toy.boom".into(),
            payload_bytes: Bytes::from(buf),
        };
        assert_eq!(r.downcast::<Boom>().unwrap().x, 7);
    }

    #[test]
    fn downcast_returns_none_on_mismatch() {
        let r = RemoteError {
            category: Category::Business,
            type_id: "other".into(),
            frames: SmallVec::new(),
            trace: TraceContext::default(),
            outer_code: Code::NotFound,
            outer_message: "x".into(),
            payload_bytes: Bytes::new(),
        };
        assert!(r.downcast::<Boom>().is_none());
    }
}
