use std::{
    fs::File,
    future::Future,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{align::checked_range, AssetError, AssetResult};

pub trait AssetReader: Clone + Send + Sync {
    fn with_offset_accumulate(&self, offset: u64) -> AssetResult<Self>;

    fn with_file(&self, file_id: u32) -> AssetResult<Self>;

    fn read_at(
        &self,
        offset: u64,
        len: u64,
    ) -> impl Future<Output = AssetResult<Bytes>> + Send + '_;
}

#[derive(Clone)]
pub struct DevDiskAssetReader {
    root_dir: PathBuf,
    file_path: PathBuf,
    base_offset: u64,
}

impl DevDiskAssetReader {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        let file_path = file_path.into();
        let root_dir = file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            root_dir,
            file_path,
            base_offset: 0,
        }
    }

    pub fn base_offset(&self) -> u64 {
        self.base_offset
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

impl AssetReader for DevDiskAssetReader {
    fn with_offset_accumulate(&self, offset: u64) -> AssetResult<Self> {
        let mut reader = self.clone();
        reader.base_offset = reader
            .base_offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        Ok(reader)
    }

    fn with_file(&self, file_id: u32) -> AssetResult<Self> {
        Ok(Self {
            root_dir: self.root_dir.clone(),
            file_path: self.root_dir.join(file_id.to_string()),
            base_offset: 0,
        })
    }

    async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
        let absolute = self
            .base_offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        let len = usize::try_from(len)?;
        let mut file = File::open(&self.file_path).map_err(io_error)?;
        file.seek(SeekFrom::Start(absolute)).map_err(io_error)?;
        let mut bytes = vec![0; len];
        file.read_exact(&mut bytes).map_err(io_error)?;
        Ok(Bytes::from(bytes))
    }
}

#[derive(Clone)]
pub struct MemoryAssetReader {
    bytes: Bytes,
    base_offset: u64,
}

impl MemoryAssetReader {
    pub fn new(bytes: Bytes) -> Self {
        Self {
            bytes,
            base_offset: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl AssetReader for MemoryAssetReader {
    fn with_offset_accumulate(&self, offset: u64) -> AssetResult<Self> {
        let mut reader = self.clone();
        reader.base_offset = reader
            .base_offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        Ok(reader)
    }

    fn with_file(&self, _file_id: u32) -> AssetResult<Self> {
        Err(AssetError::InvalidData(
            "memory asset reader does not support external files",
        ))
    }

    async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
        let absolute = self
            .base_offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        let range = checked_range(self.bytes.len() as u64, absolute, len)?;
        Ok(self.bytes.slice(range))
    }
}

fn io_error(error: std::io::Error) -> AssetError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        AssetError::UnexpectedEof
    } else {
        AssetError::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_dir() -> PathBuf {
        let name = format!(
            "file-core-reader-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(name)
    }

    #[test]
    fn dev_disk_reads_at_offset() {
        pollster::block_on(async {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("mesh0");
            fs::write(&path, b"0123456789").unwrap();

            let reader = DevDiskAssetReader::new(&path);
            assert_eq!(&reader.read_at(2, 4).await.unwrap()[..], b"2345");

            fs::remove_dir_all(dir).unwrap();
        });
    }

    #[test]
    fn dev_disk_accumulates_offsets() {
        pollster::block_on(async {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("mesh0");
            fs::write(&path, b"0123456789").unwrap();

            let reader = DevDiskAssetReader::new(&path)
                .with_offset_accumulate(2)
                .unwrap()
                .with_offset_accumulate(3)
                .unwrap();
            assert_eq!(reader.base_offset(), 5);
            assert_eq!(&reader.read_at(1, 2).await.unwrap()[..], b"67");

            fs::remove_dir_all(dir).unwrap();
        });
    }

    #[test]
    fn dev_disk_with_file_uses_root_directory_file_id() {
        pollster::block_on(async {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("mesh0");
            fs::write(&path, b"root").unwrap();
            fs::write(dir.join("13"), b"external").unwrap();

            let reader = DevDiskAssetReader::new(&path).with_file(13).unwrap();
            assert_eq!(reader.file_path(), dir.join("13").as_path());
            assert_eq!(&reader.read_at(0, 8).await.unwrap()[..], b"external");

            fs::remove_dir_all(dir).unwrap();
        });
    }

    #[test]
    fn dev_disk_offset_overflow_is_reported() {
        let reader = DevDiskAssetReader::new("mesh0")
            .with_offset_accumulate(1)
            .unwrap();
        assert!(matches!(
            reader.with_offset_accumulate(u64::MAX),
            Err(AssetError::OffsetOverflow)
        ));
    }
}
