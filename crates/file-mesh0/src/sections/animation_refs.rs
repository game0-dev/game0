use bytes::Bytes;
use file_core::{AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct AnimationRefsSectionOwned {
    pub refs: Vec<u64>,
}

impl AnimationRefsSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_animation_refs_section(self)
    }
}

#[derive(Clone)]
pub struct AnimationRefsSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    value: OnceCell<AnimationRefsSectionOwned>,
}

impl<R> AnimationRefsSectionView<R>
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

    pub async fn read_owned(&self) -> AssetResult<AnimationRefsSectionOwned> {
        Ok(self
            .value
            .get_or_try_init(|| async {
                decode_animation_refs_section(self.reader.read_at(0, u64::from(self.len)).await?)
            })
            .await?
            .clone())
    }
}

pub fn decode_animation_refs_section(bytes: Bytes) -> AssetResult<AnimationRefsSectionOwned> {
    let mut cursor = DecodeCursor::new(&bytes);
    let mut refs = Vec::with_capacity(bytes.len() / 8);
    while cursor.remaining() > 0 {
        refs.push(cursor.read_u64_le()?);
    }
    Ok(AnimationRefsSectionOwned { refs })
}

pub fn encode_animation_refs_section(section: &AnimationRefsSectionOwned) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for item in &section.refs {
        out.write_u64_le(*item);
    }
    Ok(Bytes::from(out.into_inner()))
}
