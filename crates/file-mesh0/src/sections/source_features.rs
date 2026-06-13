use bytes::Bytes;
use file_core::{
    decode_table, AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader,
};
use tokio::sync::OnceCell;

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
pub struct SourceFeaturesSectionOwned {
    pub features: Vec<Mesh0SourceFeature>,
}

impl SourceFeaturesSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_source_features_section(self)
    }
}

#[derive(Clone)]
pub struct SourceFeaturesSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    value: OnceCell<SourceFeaturesSectionOwned>,
}

impl<R> SourceFeaturesSectionView<R>
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

    pub async fn read_owned(&self) -> AssetResult<SourceFeaturesSectionOwned> {
        Ok(self
            .value
            .get_or_try_init(|| async {
                decode_source_features_section(self.reader.read_at(0, u64::from(self.len)).await?)
            })
            .await?
            .clone())
    }
}

pub fn decode_source_features_section(bytes: Bytes) -> AssetResult<SourceFeaturesSectionOwned> {
    Ok(SourceFeaturesSectionOwned {
        features: decode_table(&bytes, Mesh0SourceFeature::BYTE_SIZE, decode_source_feature)?,
    })
}

pub fn encode_source_features_section(section: &SourceFeaturesSectionOwned) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for item in &section.features {
        out.write_u32_le(item.feature_kind);
        out.write_u32_le(item.support_status);
        out.write_u32_le(item.mapped_target_kind);
        out.write_u32_le(item.mapped_target_index);
        out.write_u32_le(item.source_index);
        out.write_u32_le(item.flags);
    }
    Ok(Bytes::from(out.into_inner()))
}

fn decode_source_feature(cursor: &mut DecodeCursor<'_>) -> AssetResult<Mesh0SourceFeature> {
    Ok(Mesh0SourceFeature {
        feature_kind: cursor.read_u32_le()?,
        support_status: cursor.read_u32_le()?,
        mapped_target_kind: cursor.read_u32_le()?,
        mapped_target_index: cursor.read_u32_le()?,
        source_index: cursor.read_u32_le()?,
        flags: cursor.read_u32_le()?,
    })
}
