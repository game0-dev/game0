use bytes::Bytes;
use file_core::{AssetError, AssetResult, DecodeCursor, EncodeBuffer};

pub mod support_status {
    pub const MAPPED: u32 = 1;
    pub const IGNORED_INTENTIONAL: u32 = 2;
    pub const PRESERVED_DEBUG_ONLY: u32 = 3;
    pub const UNSUPPORTED: u32 = 4;
}

#[derive(Debug, Clone)]
pub struct Mesh0SourceFeature {
    pub feature_kind: u32,
    pub support_status: u32,
    pub mapped_target_kind: u32,
    pub mapped_target_index: u32,
    pub source_index: u32,
    pub flags: u32,
}

impl Mesh0SourceFeature {
    pub const BYTE_SIZE: usize = 24;
}

#[derive(Debug, Clone)]
pub struct SourceFeaturesSection {
    pub features: Vec<Mesh0SourceFeature>,
}

impl SourceFeaturesSection {
    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        if bytes.len() % Mesh0SourceFeature::BYTE_SIZE != 0 {
            return Err(AssetError::InvalidData("invalid source feature table size"));
        }
        let mut cursor = DecodeCursor::new(&bytes);
        let mut features = Vec::with_capacity(bytes.len() / Mesh0SourceFeature::BYTE_SIZE);
        while cursor.remaining() > 0 {
            features.push(Mesh0SourceFeature::read(&mut cursor)?);
        }
        Ok(Self { features })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        for item in &self.features {
            out.write_u32_le(item.feature_kind);
            out.write_u32_le(item.support_status);
            out.write_u32_le(item.mapped_target_kind);
            out.write_u32_le(item.mapped_target_index);
            out.write_u32_le(item.source_index);
            out.write_u32_le(item.flags);
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

impl Mesh0SourceFeature {
    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            feature_kind: cursor.read_u32_le()?,
            support_status: cursor.read_u32_le()?,
            mapped_target_kind: cursor.read_u32_le()?,
            mapped_target_index: cursor.read_u32_le()?,
            source_index: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
        })
    }
}
