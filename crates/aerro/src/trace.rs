//! OTel trace + span IDs piggy-backed on every error — see spec §7.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct TraceContext {
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
}

impl TraceContext {
    pub fn is_empty(&self) -> bool {
        self.trace_id == [0; 16] && self.span_id == [0; 8]
    }

    /// Capture from the current `tracing` span when the feature is on; zeros
    /// otherwise. No OTel layer installed also yields zeros.
    pub fn capture() -> Self {
        #[cfg(feature = "tracing")]
        {
            use opentelemetry::trace::TraceContextExt;
            use tracing_opentelemetry::OpenTelemetrySpanExt;
            let span = tracing::Span::current();
            let ctx = span.context();
            let sref = ctx.span();
            let sc = sref.span_context();
            if !sc.is_valid() {
                return Self::default();
            }
            Self {
                trace_id: sc.trace_id().to_bytes(),
                span_id: sc.span_id().to_bytes(),
            }
        }
        #[cfg(not(feature = "tracing"))]
        {
            Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(TraceContext::default().is_empty());
    }

    #[test]
    fn capture_with_no_layer_is_empty() {
        assert!(TraceContext::capture().is_empty());
    }
}
