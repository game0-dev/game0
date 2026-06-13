use bytes::Bytes;
use file_core::AssetResult;

#[derive(Debug, Clone)]
pub struct SourceDebugSection {
    pub bytes: Bytes,
}

impl SourceDebugSection {
    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        Ok(Self { bytes })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        Ok(self.bytes.clone())
    }
}
