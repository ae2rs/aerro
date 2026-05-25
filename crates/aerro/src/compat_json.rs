//! JSON compatibility layer for migration from uni-style typed errors.
//!
//! v1 scope: metadata (type_id, category, code, message, frames, trace ids)
//! is bidirectionally serialized. Typed variant data is **not** part of the
//! JSON envelope — a JSON-decoded error always surfaces as `RemoteError`,
//! never as a typed `ServiceFailure<E>`. This is intentional: the JSON shape
//! is meant to migrate uni clients off opaque blobs onto aerro, after which
//! they switch to the prost envelope (which preserves typed payloads).

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tonic::{Code, Status};

use crate::wire::encode::EncodeOptions;
use crate::{Aerro, Category, Frame, RemoteError, ServiceFailure, trace::TraceContext};

#[derive(Debug, Serialize, Deserialize)]
struct JsonEnvelope {
    type_id: String,
    category: String,
    code: u32,
    message: String,
    #[serde(default)]
    frames: Vec<JsonFrame>,
    #[serde(default, with = "hex_opt")]
    trace_id: Option<[u8; 16]>,
    #[serde(default, with = "hex_opt_8")]
    span_id: Option<[u8; 8]>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonFrame {
    service: String,
    rpc: String,
    code: u32,
    message: String,
    #[serde(default)]
    location: String,
    category: String,
}

mod hex_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<[u8; 16]>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_str(&hex_encode(b)),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<[u8; 16]>, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(s) => {
                let mut buf = [0u8; 16];
                hex_decode(&s, &mut buf).map_err(serde::de::Error::custom)?;
                Ok(Some(buf))
            }
            None => Ok(None),
        }
    }
    pub(super) fn hex_encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }
    pub(super) fn hex_decode(s: &str, into: &mut [u8]) -> Result<(), String> {
        if s.len() != into.len() * 2 {
            return Err(format!(
                "expected {} hex chars, got {}",
                into.len() * 2,
                s.len()
            ));
        }
        for (i, byte) in into.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

mod hex_opt_8 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<[u8; 8]>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_str(&super::hex_opt::hex_encode(b)),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<[u8; 8]>, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(s) => {
                let mut buf = [0u8; 8];
                super::hex_opt::hex_decode(&s, &mut buf).map_err(serde::de::Error::custom)?;
                Ok(Some(buf))
            }
            None => Ok(None),
        }
    }
}

fn category_str(c: Category) -> &'static str {
    match c {
        Category::Business => "Business",
        Category::System => "System",
        Category::Validation => "Validation",
        Category::Transport => "Transport",
    }
}

fn category_from_str(s: &str) -> Category {
    match s {
        "Business" => Category::Business,
        "Validation" => Category::Validation,
        "Transport" => Category::Transport,
        _ => Category::System,
    }
}

/// Encode a typed failure as a uni-shaped JSON `Status`.
pub fn encode_json<E: Aerro>(sf: &ServiceFailure<E>, opts: &EncodeOptions) -> Status {
    let route = opts.exposure;
    let outer_code = sf.inner.code();
    let outer_msg = if sf.inner.category() == Category::System && route != crate::Exposure::Internal
    {
        "internal error".to_string()
    } else {
        sf.inner.to_string()
    };

    let frames = if route == crate::Exposure::Public {
        Vec::new()
    } else {
        sf.frames
            .iter()
            .take(opts.max_frames as usize)
            .map(|f| JsonFrame {
                service: f.service.to_string(),
                rpc: f.rpc.to_string(),
                code: f.code as i32 as u32,
                message: f.message.to_string(),
                location: f
                    .location
                    .map(|l| format!("{}:{}", l.file(), l.line()))
                    .unwrap_or_default(),
                category: category_str(f.category).into(),
            })
            .collect()
    };

    let env = JsonEnvelope {
        type_id: Aerro::type_id(&sf.inner).to_string(),
        category: category_str(sf.inner.category()).into(),
        code: outer_code as i32 as u32,
        message: outer_msg.clone(),
        frames,
        trace_id: if sf.trace.is_empty() {
            None
        } else {
            Some(sf.trace.trace_id)
        },
        span_id: if sf.trace.is_empty() {
            None
        } else {
            Some(sf.trace.span_id)
        },
    };
    let bytes = serde_json::to_vec(&env).expect("JSON encode");
    Status::with_details(outer_code, outer_msg, bytes.into())
}

/// Decode a uni-shaped JSON `Status::details()` into a `RemoteError`. Returns
/// `None` if the bytes aren't valid JSON or don't carry a `type_id` field.
pub fn decode_json(status: &Status) -> Option<RemoteError> {
    let env: JsonEnvelope = serde_json::from_slice(status.details()).ok()?;
    let category = category_from_str(&env.category);
    let mut frames: SmallVec<[Frame; 4]> = SmallVec::new();
    for f in env.frames {
        frames.push(Frame::received(
            f.service,
            f.rpc,
            Code::from(f.code as i32),
            f.message,
            if f.location.is_empty() {
                None
            } else {
                Some(f.location)
            },
            category_from_str(&f.category),
        ));
    }
    let mut trace = TraceContext::default();
    if let Some(t) = env.trace_id {
        trace.trace_id = t;
    }
    if let Some(s) = env.span_id {
        trace.span_id = s;
    }
    Some(RemoteError::from_parts(crate::remote::RemoteErrorParts {
        category,
        type_id: env.type_id,
        frames,
        trace,
        outer_code: status.code(),
        outer_message: status.message().to_string(),
        payload_bytes: Bytes::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;

    #[test]
    fn encode_then_decode_roundtrips_metadata() {
        let sf: ServiceFailure<Boom> = Boom { x: 12 }.into();
        let st = encode_json(&sf, &EncodeOptions::default());
        let r = decode_json(&st).expect("decodes JSON");
        assert_eq!(r.type_id, "toy.boom");
        assert_eq!(r.category, Category::System);
    }

    #[test]
    fn public_drops_frames_in_json() {
        let mut sf: ServiceFailure<Boom> = Boom { x: 0 }.into();
        sf.frames.push(Frame::local(
            "svc",
            "rpc",
            Code::Internal,
            "m",
            Category::System,
        ));
        let st = encode_json(
            &sf,
            &EncodeOptions {
                exposure: crate::Exposure::Public,
                max_frames: 16,
            },
        );
        let r = decode_json(&st).unwrap();
        assert!(r.frames.is_empty());
        assert_eq!(r.outer_message, "internal error");
    }

    #[test]
    fn decode_returns_none_on_non_json_details() {
        let st = Status::internal("opaque");
        assert!(decode_json(&st).is_none());
    }
}
