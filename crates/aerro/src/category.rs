//! Variant taxonomy — see spec §8.

use tonic::Code;

use crate::Exposure;

#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Business,
    System,
    Validation,
    Transport,
}

impl Category {
    /// Exposure tier this category defaults to when a variant does not declare
    /// `#[aerro(exposure = "...")]`.
    pub fn default_exposure(self) -> Exposure {
        match self {
            Self::Business | Self::Validation => Exposure::Public,
            Self::Transport => Exposure::Trusted,
            Self::System => Exposure::Internal,
        }
    }

    /// Best-effort recovery of a category from a bare gRPC code (used when
    /// decoding a `Status` that has no aerro envelope).
    pub fn from_code(code: Code) -> Self {
        match code {
            Code::Internal | Code::Unknown | Code::DataLoss => Self::System,
            Code::Unavailable | Code::DeadlineExceeded | Code::Cancelled => Self::Transport,
            Code::InvalidArgument => Self::Validation,
            _ => Self::Business,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_default_internal() {
        assert_eq!(Category::System.default_exposure(), Exposure::Internal);
    }

    #[test]
    fn business_default_public() {
        assert_eq!(Category::Business.default_exposure(), Exposure::Public);
    }

    #[test]
    fn transport_default_trusted() {
        assert_eq!(Category::Transport.default_exposure(), Exposure::Trusted);
    }

    #[test]
    fn validation_default_public() {
        assert_eq!(Category::Validation.default_exposure(), Exposure::Public);
    }

    #[test]
    fn from_code_internal_is_system() {
        assert_eq!(Category::from_code(Code::Internal), Category::System);
    }

    #[test]
    fn from_code_unavailable_is_transport() {
        assert_eq!(Category::from_code(Code::Unavailable), Category::Transport);
    }

    #[test]
    fn from_code_invalid_arg_is_validation() {
        assert_eq!(Category::from_code(Code::InvalidArgument), Category::Validation);
    }

    #[test]
    fn from_code_not_found_is_business() {
        assert_eq!(Category::from_code(Code::NotFound), Category::Business);
    }
}
