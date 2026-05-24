//! Helpers around the generated proto types.

use crate::Category;

use super::raw;

pub fn to_proto(c: Category) -> raw::Category {
    match c {
        Category::Business => raw::Category::Business,
        Category::System => raw::Category::System,
        Category::Validation => raw::Category::Validation,
        Category::Transport => raw::Category::Transport,
    }
}

pub fn from_proto(c: raw::Category) -> Category {
    match c {
        raw::Category::Business => Category::Business,
        raw::Category::System => Category::System,
        raw::Category::Validation => Category::Validation,
        raw::Category::Transport => Category::Transport,
        raw::Category::Unspecified => Category::System,
    }
}

pub const ENVELOPE_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_all_categories() {
        for c in [
            Category::Business,
            Category::System,
            Category::Validation,
            Category::Transport,
        ] {
            assert_eq!(from_proto(to_proto(c)), c);
        }
    }
}
