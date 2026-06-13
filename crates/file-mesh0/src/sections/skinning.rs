use bytes::Bytes;
use file_core::{AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader};
use tokio::sync::OnceCell;

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
pub struct SkinningSectionOwned {
    pub skinning: Mesh0Skinning,
}

impl SkinningSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_skinning_section(self)
    }
}

#[derive(Clone)]
pub struct SkinningSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    value: OnceCell<SkinningSectionOwned>,
}

impl<R> SkinningSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub(crate) fn new(reader: OffsetAssetReader<R>, len: u32) -> Self {
        Self {
            reader,
            len,
            value: OnceCell::new(),
        }
    }

    pub async fn read_owned(&self) -> AssetResult<SkinningSectionOwned> {
        Ok(self
            .value
            .get_or_try_init(|| async {
                decode_skinning_section(self.reader.read_at(0, u64::from(self.len)).await?)
            })
            .await?
            .clone())
    }
}

pub fn decode_skinning_section(bytes: Bytes) -> AssetResult<SkinningSectionOwned> {
    let mut cursor = DecodeCursor::new(&bytes);
    Ok(SkinningSectionOwned {
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

pub fn encode_skinning_section(section: &SkinningSectionOwned) -> AssetResult<Bytes> {
    let skinning = &section.skinning;
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
