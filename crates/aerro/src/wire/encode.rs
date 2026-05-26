//! `ServiceFailure<E>` → `tonic::Status` encoding — see spec §6, §9.

use tonic::{Code, Status};

use crate::{Aerro, Category, Exposure, Frame, ServiceFailure};

use super::envelope::{ENVELOPE_VERSION, WireEnvelope, WireFrame};

/// Encoder configuration.
#[derive(Debug, Copy, Clone)]
pub struct EncodeOptions {
    /// Minimum exposure for this egress point. Variants' declared exposure is
    /// clamped *down* to this value (never up).
    pub exposure: Exposure,
    /// Maximum frames retained on the wire. Excess collapses to a synthetic
    /// `"<n> frames elided"` frame in the middle of the list.
    pub max_frames: u8,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            exposure: Exposure::Internal,
            max_frames: 16,
        }
    }
}

/// Encode a typed failure into a `tonic::Status` whose `details()` carries
/// the bincode-encoded aerro envelope.
pub fn encode<E: Aerro>(sf: &ServiceFailure<E>, opts: &EncodeOptions) -> Status {
    let route = opts.exposure;

    let outer_code = sf.inner().code();
    let outer_msg = redact_message(sf.inner(), route);

    let mut payload = Vec::new();
    if let Err(e) = sf.inner().encode_payload(route, &mut payload) {
        return Status::new(Code::Internal, format!("aerro: encode failed: {e}"));
    }

    let wire_frames = if route == Exposure::Public {
        Vec::new()
    } else {
        elide_to_cap(sf.frames(), opts.max_frames)
    };

    let trace = sf.trace();
    let env = WireEnvelope {
        version: ENVELOPE_VERSION,
        category: u8::from(sf.inner().category()),
        type_id: Aerro::type_id(sf.inner()).to_string(),
        trace_id: trace.trace_id,
        span_id: trace.span_id,
        frames: wire_frames,
        payload,
    };
    let bytes = bincode::encode_to_vec(&env, bincode::config::standard()).expect("bincode encode");

    Status::with_details(outer_code, outer_msg, bytes.into())
}

fn redact_message<E: Aerro>(inner: &E, route: Exposure) -> String {
    if inner.category() == Category::System && route != Exposure::Internal {
        "internal error".to_string()
    } else {
        inner.to_string()
    }
}

fn elide_to_cap(frames: &[Frame], cap: u8) -> Vec<WireFrame> {
    let cap = cap.max(1) as usize;
    if frames.len() <= cap {
        return frames.iter().map(to_wire_frame).collect();
    }
    let keep_front = cap / 2;
    let keep_back = cap.saturating_sub(keep_front + 1);
    let n_elided = frames.len() - keep_front - keep_back;
    let mut out = Vec::with_capacity(cap);
    out.extend(frames[..keep_front].iter().map(to_wire_frame));
    out.push(WireFrame {
        service: "...".into(),
        rpc: "elided".into(),
        code: 0,
        message: format!("{} frames elided", n_elided),
        location: String::new(),
        category: u8::from(frames[keep_front].category),
    });
    if keep_back > 0 {
        out.extend(frames[frames.len() - keep_back..].iter().map(to_wire_frame));
    }
    out
}

fn to_wire_frame(f: &Frame) -> WireFrame {
    WireFrame {
        service: f.service.to_string(),
        rpc: f.rpc.to_string(),
        code: f.code as i32 as u32,
        message: f.message.to_string(),
        location: f
            .location
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default(),
        category: u8::from(f.category),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;
    use crate::wire::envelope::WireEnvelope;
    use tonic::Code;

    #[test]
    fn system_message_redacted_at_public() {
        let sf: ServiceFailure<Boom> = Boom { x: 9 }.into();
        let st = encode(
            &sf,
            &EncodeOptions {
                exposure: Exposure::Public,
                max_frames: 16,
            },
        );
        assert_eq!(st.code(), Code::Internal);
        assert_eq!(st.message(), "internal error");
    }

    #[test]
    fn system_message_kept_at_internal() {
        let sf: ServiceFailure<Boom> = Boom { x: 9 }.into();
        let st = encode(&sf, &EncodeOptions::default());
        assert_eq!(st.code(), Code::Internal);
        assert!(st.message().starts_with("toy.boom"));
    }

    #[test]
    fn elision_keeps_cap() {
        let mut sf: ServiceFailure<Boom> = Boom { x: 0 }.into();
        for i in 0..20u32 {
            sf.frames_mut().push(Frame::local(
                "svc",
                "rpc",
                Code::Internal,
                format!("f{i}"),
                Category::System,
            ));
        }
        let st = encode(
            &sf,
            &EncodeOptions {
                exposure: Exposure::Trusted,
                max_frames: 8,
            },
        );
        let (env, _): (WireEnvelope, _) =
            bincode::decode_from_slice(st.details(), bincode::config::standard()).unwrap();
        assert_eq!(env.frames.len(), 8);
        assert!(env.frames.iter().any(|f| f.rpc == "elided"));
    }

    #[test]
    fn public_drops_frames() {
        let mut sf: ServiceFailure<Boom> = Boom { x: 0 }.into();
        sf.frames_mut().push(Frame::local(
            "svc",
            "rpc",
            Code::Internal,
            "m",
            Category::System,
        ));
        let st = encode(
            &sf,
            &EncodeOptions {
                exposure: Exposure::Public,
                max_frames: 16,
            },
        );
        let (env, _): (WireEnvelope, _) =
            bincode::decode_from_slice(st.details(), bincode::config::standard()).unwrap();
        assert!(env.frames.is_empty());
    }
}
