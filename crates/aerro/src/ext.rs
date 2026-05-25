//! Convenience extension traits for sites that want neither layer nor macro.

use tonic::Status;

use crate::wire::encode::EncodeOptions;
use crate::{Aerro, IntoStatus, RemoteError, ServiceFailure, TryFromStatus};

/// Convenience method to encode a `Result<T, E>` into a `Result<T, tonic::Status>`
/// without manually calling [`IntoStatus`].
pub trait ResultIntoStatusExt<T, E: Aerro> {
    // `tonic::Status` is ~176 bytes; we return it because tonic's RPC surface
    // requires it. The lint can't be honored here without breaking interop.
    #[allow(clippy::result_large_err)]
    /// Encode the `Err` variant into a `tonic::Status`; pass `Ok` through unchanged.
    fn into_status_ext(self, opts: &EncodeOptions) -> Result<T, Status>;
}

impl<T, E: Aerro> ResultIntoStatusExt<T, E> for Result<T, E> {
    #[allow(clippy::result_large_err)]
    fn into_status_ext(self, opts: &EncodeOptions) -> Result<T, Status> {
        self.map_err(|e| e.into_status(opts))
    }
}

/// Extension trait on `tonic::Status` to attempt typed error recovery.
pub trait StatusIntoResultExt {
    /// Try to decode the status into a [`ServiceFailure<E>`](ServiceFailure),
    /// falling back to a [`RemoteError`] if the type ID is unknown.
    fn into_aerro<E: Aerro>(self) -> Result<ServiceFailure<E>, RemoteError>;
}

impl StatusIntoResultExt for Status {
    fn into_aerro<E: Aerro>(self) -> Result<ServiceFailure<E>, RemoteError> {
        <ServiceFailure<E>>::try_from_status(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Boom;

    #[test]
    fn result_map_to_status() {
        let r: Result<(), Boom> = Err(Boom { x: 1 });
        assert!(r.into_status_ext(&EncodeOptions::default()).is_err());
    }

    #[test]
    fn status_into_aerro_recovers() {
        let st = Boom { x: 2 }.into_status(&EncodeOptions::default());
        let sf = st.into_aerro::<Boom>().unwrap();
        assert_eq!(sf.inner.x, 2);
    }
}
