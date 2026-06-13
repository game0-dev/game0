use bytes::Bytes;
use file_core::{AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct SkeletonRefsSectionOwned {
    pub refs: Vec<u64>,
}

impl SkeletonRefsSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_skeleton_refs_section(self)
    }
}

#[derive(Clone)]
pub struct SkeletonRefsSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    value: OnceCell<SkeletonRefsSectionOwned>,
}

impl<R> SkeletonRefsSectionView<R>
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

    pub async fn read_owned(&self) -> AssetResult<SkeletonRefsSectionOwned> {
        Ok(self
            .value
            .get_or_try_init(|| async {
                decode_skeleton_refs_section(self.reader.read_at(0, u64::from(self.len)).await?)
            })
            .await?
            .clone())
    }
}

pub fn decode_skeleton_refs_section(bytes: Bytes) -> AssetResult<SkeletonRefsSectionOwned> {
    let mut cursor = DecodeCursor::new(&bytes);
    let mut refs = Vec::with_capacity(bytes.len() / 8);
    while cursor.remaining() > 0 {
        refs.push(cursor.read_u64_le()?);
    }
    Ok(SkeletonRefsSectionOwned { refs })
}

pub fn encode_skeleton_refs_section(section: &SkeletonRefsSectionOwned) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for item in &section.refs {
        out.write_u64_le(*item);
    }
    Ok(Bytes::from(out.into_inner()))
}
