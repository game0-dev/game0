use bytes::Bytes;

use crate::{
    align::align_up, AssetResult, EncodeBuffer, SectionBuild, SectionRecord, SectionedAssetHeader,
};

pub struct SectionedAssetBuilder {
    version: u32,
    sections: Vec<SectionBuild>,
}

impl SectionedAssetBuilder {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            sections: Vec::new(),
        }
    }

    pub fn add_section(&mut self, section: SectionBuild) -> AssetResult<&mut Self> {
        if self
            .sections
            .iter()
            .any(|existing| existing.kind == section.kind && existing.key == section.key)
        {
            return Err(crate::AssetError::DuplicateSection {
                kind: section.kind,
                key: section.key,
            });
        }
        self.sections.push(section);
        Ok(self)
    }

    pub fn encode(&self) -> AssetResult<Bytes> {
        let section_count = u32::try_from(self.sections.len())?;
        let header_size = SectionedAssetHeader::BYTE_SIZE;
        let table_size = self.sections.len() * SectionRecord::BYTE_SIZE;
        let body_start = align_up(header_size + table_size, 8)?;
        let mut out = vec![0; body_start];
        let mut records = Vec::with_capacity(self.sections.len());

        for section in &self.sections {
            let offset = align_up(out.len(), 8)?;
            out.resize(offset, 0);
            out.extend_from_slice(&section.bytes);
            records.push(SectionRecord {
                kind: section.kind,
                key: section.key,
                flags: section.flags,
                extra: section.extra,
                offset: u64::try_from(offset)?,
                size: u64::try_from(section.bytes.len())?,
            });
        }

        let mut header = EncodeBuffer::new();
        SectionedAssetHeader {
            version: self.version,
            section_count,
        }
        .encode(&mut header);
        out[..header_size].copy_from_slice(&header.into_inner());

        let mut table = EncodeBuffer::new();
        for record in &records {
            record.encode(&mut table);
        }
        out[header_size..header_size + table_size].copy_from_slice(&table.into_inner());
        Ok(Bytes::from(out))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use bytes::Bytes;

    use crate::{
        align::is_aligned, AssetRead, AssetResult, MemoryAssetReader, SectionBuild, SectionRecord,
        SectionTable, SectionView, SectionedAssetBuilder, SectionedAssetView,
    };

    #[derive(Clone)]
    struct RecordingReader {
        inner: MemoryAssetReader,
        reads: Arc<Mutex<Vec<(u64, u64)>>>,
    }

    impl RecordingReader {
        fn new(bytes: Bytes) -> Self {
            Self {
                inner: MemoryAssetReader::new(bytes),
                reads: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl AssetRead for RecordingReader {
        async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
            self.reads.lock().unwrap().push((offset, len));
            self.inner.read_at(offset, len).await
        }
    }

    #[test]
    fn sectioned_asset_open_reads_only_header_and_table() {
        pollster::block_on(async {
            let mut builder = SectionedAssetBuilder::new(0);
            builder
                .add_section(SectionBuild {
                    kind: 1,
                    key: 0,
                    flags: 0,
                    extra: 0,
                    bytes: Bytes::from_static(b"abc"),
                })
                .unwrap();
            let reader = RecordingReader::new(builder.encode().unwrap());
            let _ = SectionedAssetView::open(reader.clone()).await.unwrap();
            assert_eq!(
                *reader.reads.lock().unwrap(),
                vec![(0, 8), (8, SectionRecord::BYTE_SIZE as u64)]
            );
        });
    }

    #[test]
    fn section_view_read_local_uses_file_absolute_offset() {
        pollster::block_on(async {
            let mut bytes = vec![0; 200];
            bytes[120..128].copy_from_slice(b"12345678");
            let reader = RecordingReader::new(Bytes::from(bytes));
            let view = SectionView::new(
                reader.clone(),
                SectionRecord {
                    kind: 1,
                    key: 0,
                    flags: 0,
                    extra: 0,
                    offset: 100,
                    size: 50,
                },
            );
            assert_eq!(&view.read_local(20, 8).await.unwrap()[..], b"12345678");
            assert_eq!(*reader.reads.lock().unwrap(), vec![(120, 8)]);
        });
    }

    #[test]
    fn section_table_rejects_duplicate_kind_key() {
        let record = SectionRecord {
            kind: 1,
            key: 2,
            flags: 0,
            extra: 0,
            offset: 0,
            size: 0,
        };
        assert!(SectionTable::new(vec![record, record]).is_err());
    }

    #[test]
    fn sectioned_asset_encode_layout_is_aligned() {
        let mut builder = SectionedAssetBuilder::new(0);
        builder
            .add_section(SectionBuild {
                kind: 1,
                key: 0,
                flags: 0,
                extra: 0,
                bytes: Bytes::from_static(b"a"),
            })
            .unwrap()
            .add_section(SectionBuild {
                kind: 2,
                key: 0,
                flags: 0,
                extra: 0,
                bytes: Bytes::from_static(b"bbbb"),
            })
            .unwrap();
        let bytes = builder.encode().unwrap();
        let first = SectionRecord::decode(&bytes[8..40]).unwrap();
        let second = SectionRecord::decode(&bytes[40..72]).unwrap();
        assert!(is_aligned(first.offset, 8));
        assert!(is_aligned(second.offset, 8));
    }
}
