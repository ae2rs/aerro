//! One hop in the call trace — see spec §7.

use std::borrow::Cow;

use tonic::Code;

use crate::Category;

#[derive(Debug, Clone)]
pub struct Frame {
    pub service: Cow<'static, str>,
    pub rpc: Cow<'static, str>,
    pub code: Code,
    pub message: Cow<'static, str>,
    pub location: Option<&'static std::panic::Location<'static>>,
    pub category: Category,
}

impl Frame {
    /// Locally captured frame — records `&'static Location` via `#[track_caller]`.
    #[track_caller]
    pub fn local(
        service: impl Into<Cow<'static, str>>,
        rpc: impl Into<Cow<'static, str>>,
        code: Code,
        message: impl Into<Cow<'static, str>>,
        category: Category,
    ) -> Self {
        Self {
            service: service.into(),
            rpc: rpc.into(),
            code,
            message: message.into(),
            location: Some(std::panic::Location::caller()),
            category,
        }
    }

    /// Frame reconstructed from the wire — has no `'static` location.
    /// The original `"file:line"` string is dropped from the in-process view;
    /// it remains on the wire for downstream hops.
    pub fn received(
        service: String,
        rpc: String,
        code: Code,
        message: String,
        location_str: Option<String>,
        category: Category,
    ) -> Self {
        let _ = location_str;
        Self {
            service: service.into(),
            rpc: rpc.into(),
            code,
            message: message.into(),
            location: None,
            category,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_captures_location() {
        let f = Frame::local("svc", "rpc", Code::Internal, "msg", Category::System);
        assert!(f.location.is_some());
    }

    #[test]
    fn received_has_no_local_location() {
        let f = Frame::received(
            "svc".into(),
            "rpc".into(),
            Code::Internal,
            "msg".into(),
            None,
            Category::System,
        );
        assert!(f.location.is_none());
    }
}
