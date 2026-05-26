//! Standard conversion traits between `ServiceFailure<E>` and `tonic::Status`.

use tonic::Status;

use crate::wire::encode::EncodeOptions;
use crate::{Aerro, RemoteError, ServiceFailure};

impl<E: Aerro> TryFrom<Status> for ServiceFailure<E> {
    type Error = RemoteError;

    fn try_from(status: Status) -> Result<Self, RemoteError> {
        crate::wire::decode::decode::<E>(status)
    }
}

impl<E: Aerro> From<ServiceFailure<E>> for Status {
    fn from(sf: ServiceFailure<E>) -> Self {
        crate::wire::encode::encode(&sf, &EncodeOptions::default())
    }
}

/// Extension trait that adds `.encode()` to every `E: Aerro`, mirroring the
/// method already available on [`ServiceFailure<E>`].
pub trait AerroEncode: Aerro + Sized {
    fn encode(self, opts: &EncodeOptions) -> Status {
        ServiceFailure::new(self).encode(opts)
    }
}

impl<E: Aerro> AerroEncode for E {}

impl<E: Aerro> ServiceFailure<E> {
    pub fn encode(self, opts: &EncodeOptions) -> Status {
        crate::wire::encode::encode(&self, opts)
    }
}

#[cfg(test)]
mod tests {
    use super::AerroEncode;
    use crate::test_support::Boom;
    use crate::wire::encode::EncodeOptions;
    use crate::{RemoteError, ServiceFailure};
    use std::convert::TryFrom;

    #[test]
    fn error_encode_method_roundtrips() {
        let status = Boom { x: 99 }.encode(&EncodeOptions::default());
        let recovered = ServiceFailure::<Boom>::try_from(status).unwrap();
        assert_eq!(recovered.inner().x, 99);
    }

    #[test]
    fn try_from_status_recovers_typed_failure() {
        let sf = ServiceFailure::new(Boom { x: 1 });
        let status: tonic::Status = sf.encode(&EncodeOptions::default());
        let recovered = ServiceFailure::<Boom>::try_from(status).unwrap();
        assert_eq!(recovered.inner().x, 1);
    }

    #[test]
    fn try_from_status_returns_remote_error_for_unknown_type() {
        let status = tonic::Status::internal("raw error");
        let result = ServiceFailure::<Boom>::try_from(status);
        assert!(matches!(result, Err(RemoteError { .. })));
    }

    #[test]
    fn service_failure_into_status_and_back_via_standard_traits() {
        let sf = ServiceFailure::new(Boom { x: 2 });
        let status: tonic::Status = sf.into();
        let recovered: ServiceFailure<Boom> = status.try_into().unwrap();
        assert_eq!(recovered.inner().x, 2);
    }

    #[test]
    fn encode_with_opts_roundtrips() {
        let sf = ServiceFailure::new(Boom { x: 3 });
        let status = sf.encode(&EncodeOptions::default());
        let recovered: ServiceFailure<Boom> = status.try_into().unwrap();
        assert_eq!(recovered.inner().x, 3);
    }
}
