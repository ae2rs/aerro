use crate::{Aerro, ServiceFailure};

/// Conversion from a downstream `ServiceFailure<T>` into a `ServiceFailure<Self>`,
/// transferring the accumulated frame chain automatically.
///
/// Implement this trait (or derive it via `#[aerro(forward)]`) on an outer error
/// enum to express that one of its variants wraps errors from a downstream service.
/// The [`From`] impl on [`ServiceFailure`] is provided automatically via a blanket
/// impl whenever this trait is implemented.
pub trait FromServiceFailure<T: Aerro>: Aerro + Sized {
    fn from_failure(sf: ServiceFailure<T>) -> ServiceFailure<Self>;
}
