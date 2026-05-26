//! Type-erased fallback for unknown wire types — see spec §5.
//!
//! Like [`crate::ServiceFailure`], the actual state lives behind a single
//! `Box` so `Result<_, RemoteError>` stays pointer-sized.

use bytes::Bytes;
use smallvec::SmallVec;
use tonic::Code;

use crate::{Aerro, Category, Frame, trace::TraceContext};

#[derive(Debug)]
pub struct RemoteErrorInner {
    pub(crate) category: Category,
    pub(crate) type_id: String,
    pub(crate) frames: SmallVec<[Frame; 4]>,
    pub(crate) trace: TraceContext,
    pub(crate) outer_code: Code,
    pub(crate) outer_message: String,
    pub(crate) payload_bytes: Bytes,
}

#[derive(Debug)]
pub struct RemoteError {
    state: Box<RemoteErrorInner>,
}

impl RemoteError {
    pub fn from_parts(inner: RemoteErrorInner) -> Self {
        Self {
            state: Box::new(inner),
        }
    }

    /// Recover a typed variant whose `type_id` is in `E::TYPE_IDS`.
    pub fn downcast<E: Aerro>(&self) -> Option<E> {
        if !E::TYPE_IDS.contains(&self.state.type_id.as_str()) {
            return None;
        }
        E::decode_payload(&self.state.type_id, &self.state.payload_bytes).ok()
    }

    pub fn category(&self) -> Category {
        self.state.category
    }

    pub fn type_id(&self) -> &str {
        &self.state.type_id
    }

    pub fn frames(&self) -> &SmallVec<[Frame; 4]> {
        &self.state.frames
    }

    pub fn trace(&self) -> &TraceContext {
        &self.state.trace
    }

    pub fn outer_code(&self) -> Code {
        self.state.outer_code
    }

    pub fn outer_message(&self) -> &str {
        &self.state.outer_message
    }
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.state.outer_code, self.state.outer_message)
    }
}

impl std::error::Error for RemoteError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;

    fn make(inner: RemoteErrorInner) -> RemoteError {
        RemoteError::from_parts(inner)
    }

    #[test]
    fn downcast_recovers_known_type() {
        use crate::Exposure;
        let mut buf = Vec::new();
        Boom { x: 7 }
            .encode_payload(Exposure::Internal, &mut buf)
            .unwrap();
        let r = make(RemoteErrorInner {
            category: Category::System,
            type_id: "toy.boom".into(),
            frames: SmallVec::new(),
            trace: TraceContext::default(),
            outer_code: Code::Internal,
            outer_message: "toy.boom".into(),
            payload_bytes: Bytes::from(buf),
        });
        assert_eq!(r.downcast::<Boom>().unwrap().x, 7);
    }

    #[test]
    fn downcast_returns_none_on_mismatch() {
        let r = make(RemoteErrorInner {
            category: Category::Business,
            type_id: "other".into(),
            frames: SmallVec::new(),
            trace: TraceContext::default(),
            outer_code: Code::NotFound,
            outer_message: "x".into(),
            payload_bytes: Bytes::new(),
        });
        assert!(r.downcast::<Boom>().is_none());
    }

    #[test]
    fn remote_error_is_pointer_sized() {
        assert_eq!(
            std::mem::size_of::<RemoteError>(),
            std::mem::size_of::<usize>()
        );
    }
}
