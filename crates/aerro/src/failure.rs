//! `ServiceFailure<E>` — typed error plus accumulated trace.
//!
//! The actual state lives behind a single `Box<ServiceFailureInner<E>>` so that
//! `Result<T, ServiceFailure<E>>` stays pointer-sized on the happy path — the
//! `SmallVec<[Frame; 4]>` and `TraceContext` would otherwise inflate every
//! `Result` carried up the call stack. Errors are cold; we pay one allocation
//! on the error path to keep the success path cheap.

use smallvec::SmallVec;

use crate::{Aerro, Frame, trace::TraceContext};
use crate::traits::FromServiceFailure;

#[derive(Debug)]
pub(crate) struct ServiceFailureInner<E: Aerro> {
    pub(crate) inner: E,
    pub(crate) frames: SmallVec<[Frame; 4]>,
    pub(crate) trace: TraceContext,
}

#[derive(Debug)]
pub struct ServiceFailure<E: Aerro> {
    state: Box<ServiceFailureInner<E>>,
}

impl<E: Aerro> ServiceFailure<E> {
    pub fn new(inner: E) -> Self {
        Self {
            state: Box::new(ServiceFailureInner {
                inner,
                frames: SmallVec::new(),
                trace: TraceContext::capture(),
            }),
        }
    }

    pub fn from_parts(inner: E, frames: SmallVec<[Frame; 4]>, trace: TraceContext) -> Self {
        Self {
            state: Box::new(ServiceFailureInner {
                inner,
                frames,
                trace,
            }),
        }
    }

    pub fn push_frame(&mut self, f: Frame) {
        self.state.frames.push(f);
    }

    /// Consume `self` and return the typed error, dropping frames and trace.
    pub fn into_inner(self) -> E {
        self.state.inner
    }

    /// Consume `self` and return all three components.
    pub fn into_parts(self) -> (E, SmallVec<[Frame; 4]>, TraceContext) {
        let ServiceFailureInner {
            inner,
            frames,
            trace,
        } = *self.state;
        (inner, frames, trace)
    }

    pub fn inner(&self) -> &E {
        &self.state.inner
    }

    pub fn inner_mut(&mut self) -> &mut E {
        &mut self.state.inner
    }

    pub fn frames(&self) -> &SmallVec<[Frame; 4]> {
        &self.state.frames
    }

    pub fn frames_mut(&mut self) -> &mut SmallVec<[Frame; 4]> {
        &mut self.state.frames
    }

    pub fn trace(&self) -> &TraceContext {
        &self.state.trace
    }

    pub fn trace_mut(&mut self) -> &mut TraceContext {
        &mut self.state.trace
    }
}

impl<E: Aerro> std::fmt::Display for ServiceFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.state.inner, f)
    }
}

impl<E: Aerro> std::error::Error for ServiceFailure<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.state.inner.source()
    }
}

impl<E: Aerro> From<E> for ServiceFailure<E> {
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

impl<E: Aerro> ServiceFailure<E> {
    /// Convert this failure into a `ServiceFailure<O>` by way of a forwarding
    /// variant on `O` that was declared with `#[aerro(forward)]`.
    ///
    /// Frames and trace context are preserved — no manual [`Frame::local`] push
    /// needed at the forwarding boundary.
    pub fn forward<O: Aerro + FromServiceFailure<E>>(self) -> ServiceFailure<O> {
        O::from_failure(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;

    #[test]
    fn from_e_constructs_failure_with_empty_frames() {
        let sf: ServiceFailure<Boom> = Boom { x: 1 }.into();
        assert!(sf.frames().is_empty());
        assert!(sf.trace().is_empty());
    }

    #[test]
    fn service_failure_is_pointer_sized() {
        // Whole point of boxing: keep `Result<T, ServiceFailure<E>>` small.
        assert_eq!(
            std::mem::size_of::<ServiceFailure<Boom>>(),
            std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn into_inner_yields_typed_error() {
        let sf: ServiceFailure<Boom> = Boom { x: 42 }.into();
        let b = sf.into_inner();
        assert_eq!(b.x, 42);
    }
}
