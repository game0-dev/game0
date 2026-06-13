use bytes::Bytes;
use file_core::{AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct EffectRefsSectionOwned {
    pub refs: Vec<u64>,
}

impl EffectRefsSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_effect_refs_section(self)
    }
}

#[derive(Clone)]
pub struct EffectRefsSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    value: OnceCell<EffectRefsSectionOwned>,
}

impl<R> EffectRefsSectionView<R>
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

    pub async fn read_owned(&self) -> AssetResult<EffectRefsSectionOwned> {
        Ok(self
            .value
            .get_or_try_init(|| async {
                decode_effect_refs_section(self.reader.read_at(0, u64::from(self.len)).await?)
            })
            .await?
            .clone())
    }
}

pub fn decode_effect_refs_section(bytes: Bytes) -> AssetResult<EffectRefsSectionOwned> {
    let mut cursor = DecodeCursor::new(&bytes);
    let mut refs = Vec::with_capacity(bytes.len() / 8);
    while cursor.remaining() > 0 {
        refs.push(cursor.read_u64_le()?);
    }
    Ok(EffectRefsSectionOwned { refs })
}

pub fn encode_effect_refs_section(section: &EffectRefsSectionOwned) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for item in &section.refs {
        out.write_u64_le(*item);
    }
    Ok(Bytes::from(out.into_inner()))
}
