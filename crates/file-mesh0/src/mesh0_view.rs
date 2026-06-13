use file_core::{
    AssetError, AssetRead, AssetReadExt, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader,
};
use tokio::sync::OnceCell;

use crate::{mesh0_owned::Mesh0Owned, sections::*};

pub const SECTION_TABLE_ITEM_BYTE_SIZE: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mesh0Header {
    pub version: u32,
    pub section_count: u32,
}

impl Mesh0Header {
    pub const BYTE_SIZE: usize = 8;

    pub fn new(section_count: u32) -> Self {
        Self {
            version: MESH0_VERSION_0,
            section_count,
        }
    }

    pub fn read(bytes: &[u8]) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(bytes);
        Ok(Self {
            version: cursor.read_u32_le()?,
            section_count: cursor.read_u32_le()?,
        })
    }

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.version);
        out.write_u32_le(self.section_count);
    }
}

#[derive(Clone)]
pub struct SectionTableItem<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    section_type: u32,
    offset: u32,
    len: u32,
    reader: OffsetAssetReader<R>,
    view: OnceCell<SectionView<R>>,
}

impl<R> SectionTableItem<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub const BYTE_SIZE: usize = SECTION_TABLE_ITEM_BYTE_SIZE;

    pub fn section_type(&self) -> u32 {
        self.section_type
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_type(&self, section_type: u32) -> bool {
        self.section_type == section_type
    }

    pub async fn read_section_view(&self) -> AssetResult<&SectionView<R>> {
        self.view
            .get_or_try_init(|| async {
                SectionView::read(self.reader.clone(), self.section_type, self.len).await
            })
            .await
    }

    pub(crate) fn read(cursor: &mut DecodeCursor<'_>, reader: R) -> AssetResult<Self> {
        let section_type = cursor.read_u32_le()?;
        let offset = cursor.read_u32_le()?;
        let len = cursor.read_u32_le()?;
        Ok(Self {
            section_type,
            offset,
            len,
            reader: reader.with_base_offset(u64::from(offset)),
            view: OnceCell::new(),
        })
    }

    pub(crate) fn write(out: &mut EncodeBuffer, section_type: u32, offset: u32, len: u32) {
        out.write_u32_le(section_type);
        out.write_u32_le(offset);
        out.write_u32_le(len);
    }
}

#[derive(Clone)]
pub struct Mesh0View<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    version: u32,
    section_items: Vec<SectionTableItem<R>>,
}

impl<R> Mesh0View<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn open(reader: R) -> AssetResult<Self> {
        let header_bytes = reader.read_at(0, Mesh0Header::BYTE_SIZE as u64).await?;
        let header = Mesh0Header::read(&header_bytes)?;
        if header.version != MESH0_VERSION_0 {
            return Err(AssetError::UnsupportedFormatVersion(header.version));
        }

        let table_len = u64::from(header.section_count)
            .checked_mul(SectionTableItem::<R>::BYTE_SIZE as u64)
            .ok_or(AssetError::OffsetOverflow)?;
        let table_bytes = reader
            .read_at(Mesh0Header::BYTE_SIZE as u64, table_len)
            .await?;
        let mut cursor = DecodeCursor::new(&table_bytes);
        let mut section_items = Vec::with_capacity(header.section_count as usize);
        while cursor.remaining() > 0 {
            section_items.push(SectionTableItem::read(&mut cursor, reader.clone())?);
        }
        Ok(Self {
            version: header.version,
            section_items,
        })
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn sections(&self) -> &[SectionTableItem<R>] {
        &self.section_items
    }

    pub fn sections_by_type(
        &self,
        section_type: u32,
    ) -> impl Iterator<Item = &SectionTableItem<R>> {
        self.section_items
            .iter()
            .filter(move |section| section.is_type(section_type))
    }

    pub async fn read_owned(&self) -> AssetResult<Mesh0Owned> {
        let mut sections = Vec::with_capacity(self.section_items.len());
        for item in &self.section_items {
            sections.push(item.read_section_view().await?.read_owned().await?);
        }
        Ok(Mesh0Owned { sections })
    }
}
