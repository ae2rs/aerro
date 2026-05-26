//! Pure-bincode wire envelope — replaces the prost/protobuf encoding (v2).

use crate::Category;

pub(crate) const ENVELOPE_VERSION: u32 = 2;

#[derive(Debug, bincode::Encode, bincode::Decode)]
pub(crate) struct WireEnvelope {
    pub version: u32,
    pub category: u8,
    pub type_id: String,
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub frames: Vec<WireFrame>,
    pub payload: Vec<u8>,
}

#[derive(Debug, bincode::Encode, bincode::Decode)]
pub(crate) struct WireFrame {
    pub service: String,
    pub rpc: String,
    pub code: u32,
    pub message: String,
    pub location: String,
    pub category: u8,
}

impl From<Category> for u8 {
    fn from(c: Category) -> u8 {
        match c {
            Category::Business => 1,
            Category::System => 2,
            Category::Validation => 3,
            Category::Transport => 4,
        }
    }
}

impl TryFrom<u8> for Category {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, ()> {
        match v {
            1 => Ok(Category::Business),
            2 => Ok(Category::System),
            3 => Ok(Category::Validation),
            4 => Ok(Category::Transport),
            _ => Err(()),
        }
    }
}

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
            let byte: u8 = c.into();
            assert_eq!(Category::try_from(byte).unwrap(), c);
        }
    }
}
