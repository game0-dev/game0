use crate::{AssetError, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionedAssetHeader {
    pub version: u32,
    pub section_count: u32,
}

impl SectionedAssetHeader {
    pub const BYTE_SIZE: usize = 8;

    pub fn decode(bytes: &[u8]) -> AssetResult<Self> {
        if bytes.len() != Self::BYTE_SIZE {
            return Err(AssetError::UnexpectedEof);
        }
        let mut cursor = DecodeCursor::new(bytes);
        Ok(Self {
            version: cursor.read_u32_le()?,
            section_count: cursor.read_u32_le()?,
        })
    }

    pub fn encode(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.version);
        out.write_u32_le(self.section_count);
    }
}
