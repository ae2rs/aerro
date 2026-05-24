//! `ServiceFailure<E>` — typed error plus accumulated trace.

use smallvec::SmallVec;

use crate::{Aerro, Frame, trace::TraceContext};

#[derive(Debug)]
pub struct ServiceFailure<E: Aerro> {
    pub inner: E,
    pub frames: SmallVec<[Frame; 4]>,
    pub trace: TraceContext,
}

impl<E: Aerro> ServiceFailure<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            frames: SmallVec::new(),
            trace: TraceContext::capture(),
        }
    }

    pub fn push_frame(&mut self, f: Frame) {
        self.frames.push(f);
    }
}

impl<E: Aerro> std::fmt::Display for ServiceFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl<E: Aerro> std::error::Error for ServiceFailure<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
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
}
