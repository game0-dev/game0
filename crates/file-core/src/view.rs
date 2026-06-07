use bytes::Bytes;

use crate::{
    align::checked_range, AssetRead, AssetResult, DecodeCursor, SectionRecord, SectionTable,
    SectionedAssetHeader,
};

#[derive(Clone)]
pub struct SectionView<R> {
    reader: R,
    record: SectionRecord,
}

impl<R> SectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub fn new(reader: R, record: SectionRecord) -> Self {
        Self { reader, record }
    }

    pub fn record(&self) -> &SectionRecord {
        &self.record
    }

    pub async fn read_all(&self) -> AssetResult<Bytes> {
        self.reader
            .read_at(self.record.offset, self.record.size)
            .await
    }

    pub async fn read_local(&self, offset: u64, size: u64) -> AssetResult<Bytes> {
        checked_range(self.record.size, offset, size)?;
        let absolute = self
            .record
            .offset
            .checked_add(offset)
            .ok_or(crate::AssetError::OffsetOverflow)?;
        self.reader.read_at(absolute, size).await
    }
}

#[derive(Clone)]
pub struct SectionedAssetView<R> {
    reader: R,
    header: SectionedAssetHeader,
    table: SectionTable,
}

impl<R> SectionedAssetView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn open(reader: R) -> AssetResult<Self> {
        let header_bytes = reader
            .read_at(0, SectionedAssetHeader::BYTE_SIZE as u64)
            .await?;
        let header = SectionedAssetHeader::decode(&header_bytes)?;
        let table_len = u64::from(header.section_count)
            .checked_mul(SectionRecord::BYTE_SIZE as u64)
            .ok_or(crate::AssetError::OffsetOverflow)?;
        let table_bytes = reader
            .read_at(SectionedAssetHeader::BYTE_SIZE as u64, table_len)
            .await?;
        let mut cursor = DecodeCursor::new(&table_bytes);
        let mut records = Vec::with_capacity(header.section_count as usize);
        while cursor.remaining() > 0 {
            records.push(SectionRecord::decode(
                cursor.read_bytes(SectionRecord::BYTE_SIZE)?,
            )?);
        }
        let table = SectionTable::new(records)?;
        Ok(Self {
            reader,
            header,
            table,
        })
    }

    pub fn header(&self) -> &SectionedAssetHeader {
        &self.header
    }

    pub fn table(&self) -> &SectionTable {
        &self.table
    }

    pub fn section(&self, kind: u32, key: u32) -> AssetResult<SectionView<R>> {
        let record = *self.table.require(kind, key)?;
        Ok(self.section_from_record(record))
    }

    pub fn section_from_record(&self, record: SectionRecord) -> SectionView<R> {
        SectionView::new(self.reader.clone(), record)
    }

    pub fn section_at(&self, index: usize) -> AssetResult<SectionView<R>> {
        let record = *self
            .table
            .records()
            .get(index)
            .ok_or(crate::AssetError::RangeOutOfBounds)?;
        Ok(self.section_from_record(record))
    }

    pub fn sections(&self) -> impl Iterator<Item = SectionView<R>> + '_ {
        self.table
            .records()
            .iter()
            .copied()
            .map(|record| self.section_from_record(record))
    }
}
