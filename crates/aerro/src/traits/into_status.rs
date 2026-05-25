//! `IntoStatus` — convert anything implementing `Aerro` into a `tonic::Status`.

use tonic::Status;

use crate::wire::encode::{EncodeOptions, encode};
use crate::{Aerro, ServiceFailure};

pub trait IntoStatus {
    fn into_status(self, opts: &EncodeOptions) -> Status;
}

impl<E: Aerro> IntoStatus for E {
    fn into_status(self, opts: &EncodeOptions) -> Status {
        encode(&ServiceFailure::new(self), opts)
    }
}

impl<E: Aerro> IntoStatus for ServiceFailure<E> {
    fn into_status(self, opts: &EncodeOptions) -> Status {
        encode(&self, opts)
    }
}
