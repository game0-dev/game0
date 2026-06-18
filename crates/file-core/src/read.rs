use std::future::Future;

use bytes::Bytes;

use crate::{align::checked_range, AssetError, AssetResult};

pub trait AssetRead {
    fn read_at(
        &self,
        offset: u64,
        len: u64,
    ) -> impl Future<Output = AssetResult<Bytes>> + Send + '_;
}

pub trait AssetReadExt: AssetRead + Sized {
    fn with_offset_accumulate(self, offset: u64) -> OffsetAssetReader<Self> {
        OffsetAssetReader {
            inner: self,
            base_offset: offset,
            file_id: 0,
        }
    }

    fn with_file(self, file_id: u32) -> OffsetAssetReader<Self> {
        OffsetAssetReader {
            inner: self,
            base_offset: 0,
            file_id,
        }
    }
}

impl<T> AssetReadExt for T where T: AssetRead + Sized {}

#[derive(Clone)]
pub struct OffsetAssetReader<R> {
    inner: R,
    base_offset: u64,
    file_id: u32,
}

impl<R> OffsetAssetReader<R> {
    pub fn base_offset(&self) -> u64 {
        self.base_offset
    }

    pub fn file_id(&self) -> u32 {
        self.file_id
    }

    pub fn is_external(&self) -> bool {
        self.file_id != 0
    }

    pub fn with_offset_accumulate(mut self, offset: u64) -> Self {
        self.base_offset = self
            .base_offset
            .checked_add(offset)
            .expect("asset reader offset overflow");
        self
    }

    pub fn with_file(mut self, file_id: u32) -> Self {
        self.file_id = file_id;
        self
    }
}

impl<R> AssetRead for OffsetAssetReader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
        let absolute = self
            .base_offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        self.inner.read_at(absolute, len).await
    }
}

#[derive(Clone)]
pub struct MemoryAssetReader {
    bytes: Bytes,
}

impl MemoryAssetReader {
    pub fn new(bytes: Bytes) -> Self {
        Self { bytes }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl AssetRead for MemoryAssetReader {
    async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
        let range = checked_range(self.bytes.len() as u64, offset, len)?;
        Ok(self.bytes.slice(range))
    }
}
