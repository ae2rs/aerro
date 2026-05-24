//! Convenience extension traits for sites that want neither layer nor macro.

use tonic::Status;

use crate::{Aerro, IntoStatus, RemoteError, ServiceFailure, TryFromStatus};
use crate::wire::encode::EncodeOptions;

pub trait ResultIntoStatusExt<T, E: Aerro> {
    fn into_status_ext(self, opts: &EncodeOptions) -> Result<T, Status>;
}

impl<T, E: Aerro> ResultIntoStatusExt<T, E> for Result<T, E> {
    fn into_status_ext(self, opts: &EncodeOptions) -> Result<T, Status> {
        self.map_err(|e| e.into_status(opts))
    }
}

pub trait StatusIntoResultExt {
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
