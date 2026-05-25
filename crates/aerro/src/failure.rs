//! `ServiceFailure<E>` — typed error plus accumulated trace.
//!
//! The actual state lives behind a single `Box<ServiceFailureInner<E>>` so that
//! `Result<T, ServiceFailure<E>>` stays pointer-sized on the happy path — the
//! `SmallVec<[Frame; 4]>` and `TraceContext` would otherwise inflate every
//! `Result` carried up the call stack. Errors are cold; we pay one allocation
//! on the error path to keep the success path cheap.

use smallvec::SmallVec;

use crate::{Aerro, Frame, trace::TraceContext};

/// Boxed state of [`ServiceFailure`]. Exposed only so that field access
/// through `Deref`/`DerefMut` (`sf.inner`, `sf.frames`, `sf.trace`) names a
/// public type; you almost never write this name directly.
#[derive(Debug)]
pub struct ServiceFailureInner<E: Aerro> {
    pub inner: E,
    pub frames: SmallVec<[Frame; 4]>,
    pub trace: TraceContext,
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
}

impl<E: Aerro> std::ops::Deref for ServiceFailure<E> {
    type Target = ServiceFailureInner<E>;
    fn deref(&self) -> &ServiceFailureInner<E> {
        &self.state
    }
}

impl<E: Aerro> std::ops::DerefMut for ServiceFailure<E> {
    fn deref_mut(&mut self) -> &mut ServiceFailureInner<E> {
        &mut self.state
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;

    #[test]
    fn from_e_constructs_failure_with_empty_frames() {
        let sf: ServiceFailure<Boom> = Boom { x: 1 }.into();
        assert!(sf.frames.is_empty());
        assert!(sf.trace.is_empty());
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
