//! `tonic::Status` → `ServiceFailure<E>` decoding — the client-side counterpart to
//! [`encode`](crate::wire::encode::encode).

use bytes::Bytes;
use prost::Message;
use smallvec::SmallVec;
use tonic::{Code, Status};

use crate::{Aerro, Category, Frame, RemoteError, ServiceFailure, trace::TraceContext};

use super::envelope::from_proto;
use super::raw;

/// Decode a `tonic::Status` into a [`ServiceFailure<E>`](crate::ServiceFailure).
///
/// Returns `Err(RemoteError)` when the envelope's type ID is not in `E::TYPE_IDS`.
/// The returned `RemoteError` still carries the wire envelope so it can be re-encoded
/// or forwarded without data loss.
pub fn decode<E: Aerro>(status: Status) -> Result<ServiceFailure<E>, RemoteError> {
    let details = status.details();
    if details.is_empty() {
        return Err(transport_remote_error(&status));
    }
    let env = match raw::Envelope::decode(details) {
        Ok(e) => e,
        Err(_) => return Err(transport_remote_error(&status)),
    };

    if !E::TYPE_IDS.contains(&env.type_id.as_str()) {
        return Err(into_remote_error(env, &status));
    }

    let inner = match E::decode_payload(&env.type_id, &env.payload) {
        Ok(v) => v,
        Err(_) => return Err(into_remote_error(env, &status)),
    };
    let frames = decode_frames(&env.frames);
    let trace = decode_trace(&env.trace_id, &env.span_id);
    Ok(ServiceFailure::from_parts(inner, frames, trace))
}

fn into_remote_error(env: raw::Envelope, status: &Status) -> RemoteError {
    let category =
        from_proto(raw::Category::try_from(env.category).unwrap_or(raw::Category::System));
    let trace = decode_trace(&env.trace_id, &env.span_id);
    let frames = decode_frames(&env.frames);
    RemoteError::from_parts(crate::remote::RemoteErrorParts {
        category,
        type_id: env.type_id,
        frames,
        trace,
        outer_code: status.code(),
        outer_message: status.message().to_string(),
        payload_bytes: Bytes::from(env.payload.to_vec()),
    })
}

fn transport_remote_error(status: &Status) -> RemoteError {
    RemoteError::from_parts(crate::remote::RemoteErrorParts {
        category: Category::Transport,
        type_id: "aerro.transport".into(),
        frames: SmallVec::new(),
        trace: TraceContext::default(),
        outer_code: status.code(),
        outer_message: status.message().to_string(),
        payload_bytes: Bytes::new(),
    })
}

fn decode_frames(frames: &[raw::Frame]) -> SmallVec<[Frame; 4]> {
    let mut out = SmallVec::new();
    for f in frames {
        let cat = from_proto(raw::Category::try_from(f.category).unwrap_or(raw::Category::System));
        let code = code_from_u32(f.code);
        let loc = if f.location.is_empty() {
            None
        } else {
            Some(f.location.clone())
        };
        out.push(Frame::received(
            f.service.clone(),
            f.rpc.clone(),
            code,
            f.message.clone(),
            loc,
            cat,
        ));
    }
    out
}

fn code_from_u32(c: u32) -> Code {
    // tonic::Code is `#[repr(i32)]` over the gRPC code numbers 0..=16.
    Code::from(c as i32)
}

fn decode_trace(trace_id: &[u8], span_id: &[u8]) -> TraceContext {
    let mut t = TraceContext::default();
    if trace_id.len() == 16 {
        t.trace_id.copy_from_slice(trace_id);
    }
    if span_id.len() == 8 {
        t.span_id.copy_from_slice(span_id);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;
    use crate::wire::encode::{EncodeOptions, encode};

    #[test]
    fn roundtrip_known_type() {
        let sf: ServiceFailure<Boom> = Boom { x: 13 }.into();
        let st = encode(&sf, &EncodeOptions::default());
        let back = decode::<Boom>(st).expect("known type round-trips");
        assert_eq!(back.inner.x, 13);
    }

    #[test]
    fn unknown_type_falls_back_to_remote_error() {
        let sf: ServiceFailure<Boom> = Boom { x: 5 }.into();
        let st = encode(&sf, &EncodeOptions::default());

        #[derive(Debug, thiserror::Error)]
        #[error("other")]
        struct Other;
        impl Aerro for Other {
            const TYPE_IDS: &'static [&'static str] = &["other.thing"];
            fn type_id(&self) -> &'static str {
                "other.thing"
            }
            fn category(&self) -> Category {
                Category::Business
            }
            fn code(&self) -> Code {
                Code::NotFound
            }
            fn encode_payload(&self, _: crate::Exposure, _: &mut Vec<u8>) -> Result<(), crate::EncodeError> { Ok(()) }
            fn decode_payload(_: &str, _: &[u8]) -> Result<Self, crate::DecodeError> {
                Err(crate::DecodeError::Missing)
            }
        }
        let r = decode::<Other>(st).expect_err("type_id mismatch");
        assert_eq!(r.type_id, "toy.boom");
        assert_eq!(r.downcast::<Boom>().unwrap().x, 5);
    }

    #[test]
    fn bare_status_becomes_transport_remote_error() {
        let st = Status::unavailable("backend down");
        let r = decode::<Boom>(st).err().unwrap();
        assert_eq!(r.category, Category::Transport);
        assert_eq!(r.outer_code, Code::Unavailable);
    }
}
