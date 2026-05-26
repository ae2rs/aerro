//! `IntoStatus` — convert anything implementing `Aerro` into a `tonic::Status`.

use tonic::Status;

use crate::wire::encode::{EncodeOptions, encode};
use crate::{Aerro, ServiceFailure};

/// Convert a typed error or [`ServiceFailure`] into a `tonic::Status` for
/// transmission over gRPC.
///
/// Encoding applies exposure redaction and embeds the aerro envelope in the
/// status `details()` field. See [`EncodeOptions`] to control the egress
/// exposure tier and frame cap.
pub trait IntoStatus {
    /// Encode `self` into a `tonic::Status` using the given options.
    fn into_status(self, opts: &EncodeOptions) -> Status;

    /// Encode `self` into a `tonic::Status` using [`EncodeOptions::default()`].
    fn into_status_default(self) -> Status
    where
        Self: Sized,
    {
        self.into_status(&EncodeOptions::default())
    }
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
