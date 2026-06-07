use std::future::Future;

use bytes::Bytes;

use crate::{align::checked_range, AssetResult};

pub trait AssetRead {
    fn read_at(
        &self,
        offset: u64,
        len: u64,
    ) -> impl Future<Output = AssetResult<Bytes>> + Send + '_;
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
