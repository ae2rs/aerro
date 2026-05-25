//! Exposure tiers — see spec §9.
//!
//! Ordered `Internal < Trusted < Public`. The encoder clamps each variant's
//! declared exposure down to the route minimum; it never upgrades.

#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Exposure {
    Internal,
    Trusted,
    Public,
}

impl Exposure {
    /// Lower of two exposures. Used by the encoder to clamp a variant's
    /// declared exposure down to the route's minimum.
    pub fn clamp(self, route_min: Exposure) -> Exposure {
        if (self as u8) < (route_min as u8) {
            self
        } else {
            route_min
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_internal_lt_trusted_lt_public() {
        assert!(Exposure::Internal < Exposure::Trusted);
        assert!(Exposure::Trusted < Exposure::Public);
    }

    #[test]
    fn clamp_never_upgrades() {
        assert_eq!(
            Exposure::Internal.clamp(Exposure::Public),
            Exposure::Internal
        );
        assert_eq!(
            Exposure::Public.clamp(Exposure::Internal),
            Exposure::Internal
        );
        assert_eq!(Exposure::Trusted.clamp(Exposure::Public), Exposure::Trusted);
    }
}
