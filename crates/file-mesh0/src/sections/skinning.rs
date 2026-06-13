use bytes::Bytes;
use file_core::{AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
pub struct Mesh0Skinning {
    pub skeleton_asset: u64,
    pub flags: u32,
    pub joint_count_hint: u32,
    pub max_weights_per_vertex: u32,
    pub joint_index_format: u32,
    pub weight_format: u32,
    pub source_bone_count: u32,
    pub source_key_bone_count: u32,
}

#[derive(Debug, Clone)]
pub struct SkinningSection {
    pub skinning: Mesh0Skinning,
}

impl SkinningSection {
    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(&bytes);
        Ok(Self {
            skinning: Mesh0Skinning {
                skeleton_asset: cursor.read_u64_le()?,
                flags: cursor.read_u32_le()?,
                joint_count_hint: cursor.read_u32_le()?,
                max_weights_per_vertex: cursor.read_u32_le()?,
                joint_index_format: cursor.read_u32_le()?,
                weight_format: cursor.read_u32_le()?,
                source_bone_count: cursor.read_u32_le()?,
                source_key_bone_count: cursor.read_u32_le()?,
            },
        })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let skinning = &self.skinning;
        let mut out = EncodeBuffer::new();
        out.write_u64_le(skinning.skeleton_asset);
        out.write_u32_le(skinning.flags);
        out.write_u32_le(skinning.joint_count_hint);
        out.write_u32_le(skinning.max_weights_per_vertex);
        out.write_u32_le(skinning.joint_index_format);
        out.write_u32_le(skinning.weight_format);
        out.write_u32_le(skinning.source_bone_count);
        out.write_u32_le(skinning.source_key_bone_count);
        Ok(Bytes::from(out.into_inner()))
    }
}
