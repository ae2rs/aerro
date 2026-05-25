//! `TryFromStatus` — recover a typed failure from a `tonic::Status`.

use tonic::Status;

use crate::wire::decode::decode;
use crate::{Aerro, RemoteError, ServiceFailure};

/// Attempt to decode a `tonic::Status` back into a typed
/// [`ServiceFailure<E>`](crate::ServiceFailure).
///
/// Returns `Err(RemoteError)` if the embedded type ID is not in `E::TYPE_IDS`
/// (the error came from an unknown service or error type).
pub trait TryFromStatus<E: Aerro>: Sized {
    /// Decode a status, recovering the original `E` variant when the type ID matches.
    fn try_from_status(status: Status) -> Result<ServiceFailure<E>, RemoteError>;
}

impl<E: Aerro> TryFromStatus<E> for ServiceFailure<E> {
    fn try_from_status(status: Status) -> Result<ServiceFailure<E>, RemoteError> {
        decode::<E>(status)
    }
}
