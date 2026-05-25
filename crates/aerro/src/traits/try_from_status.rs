//! `TryFromStatus` — recover a typed failure from a `tonic::Status`.

use tonic::Status;

use crate::wire::decode::decode;
use crate::{Aerro, RemoteError, ServiceFailure};

pub trait TryFromStatus<E: Aerro>: Sized {
    fn try_from_status(status: Status) -> Result<ServiceFailure<E>, RemoteError>;
}

impl<E: Aerro> TryFromStatus<E> for ServiceFailure<E> {
    fn try_from_status(status: Status) -> Result<ServiceFailure<E>, RemoteError> {
        decode::<E>(status)
    }
}
