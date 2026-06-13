use bytes::Bytes;
use file_core::{AssetRead, AssetResult, OffsetAssetReader};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct SourceDebugSectionOwned {
    pub bytes: Bytes,
}

impl SourceDebugSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_source_debug_section(self)
    }
}

#[derive(Clone)]
pub struct SourceDebugSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    value: OnceCell<SourceDebugSectionOwned>,
}

impl<R> SourceDebugSectionView<R>
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

    pub async fn read_owned(&self) -> AssetResult<SourceDebugSectionOwned> {
        Ok(self
            .value
            .get_or_try_init(|| async {
                decode_source_debug_section(self.reader.read_at(0, u64::from(self.len)).await?)
            })
            .await?
            .clone())
    }
}

pub fn decode_source_debug_section(bytes: Bytes) -> AssetResult<SourceDebugSectionOwned> {
    Ok(SourceDebugSectionOwned { bytes })
}

pub fn encode_source_debug_section(section: &SourceDebugSectionOwned) -> AssetResult<Bytes> {
    Ok(section.bytes.clone())
}
