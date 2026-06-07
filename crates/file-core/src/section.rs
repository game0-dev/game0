use std::collections::HashSet;

use bytes::Bytes;

use crate::{AssetError, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionRecord {
    pub kind: u32,
    pub key: u32,
    pub flags: u32,
    pub extra: u32,
    pub offset: u64,
    pub size: u64,
}

impl SectionRecord {
    pub const BYTE_SIZE: usize = 32;

    pub fn decode(bytes: &[u8]) -> AssetResult<Self> {
        if bytes.len() != Self::BYTE_SIZE {
            return Err(AssetError::UnexpectedEof);
        }
        let mut cursor = DecodeCursor::new(bytes);
        Ok(Self {
            kind: cursor.read_u32_le()?,
            key: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
            extra: cursor.read_u32_le()?,
            offset: cursor.read_u64_le()?,
            size: cursor.read_u64_le()?,
        })
    }

    pub fn encode(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.kind);
        out.write_u32_le(self.key);
        out.write_u32_le(self.flags);
        out.write_u32_le(self.extra);
        out.write_u64_le(self.offset);
        out.write_u64_le(self.size);
    }

    pub fn end_offset(&self) -> AssetResult<u64> {
        self.offset
            .checked_add(self.size)
            .ok_or(AssetError::OffsetOverflow)
    }
}

#[derive(Debug, Clone)]
pub struct SectionTable {
    records: Vec<SectionRecord>,
}

impl SectionTable {
    pub fn new(records: Vec<SectionRecord>) -> AssetResult<Self> {
        let mut seen = HashSet::new();
        for record in &records {
            if !seen.insert((record.kind, record.key)) {
                return Err(AssetError::DuplicateSection {
                    kind: record.kind,
                    key: record.key,
                });
            }
            record.end_offset()?;
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> &[SectionRecord] {
        &self.records
    }

    pub fn get(&self, kind: u32, key: u32) -> Option<&SectionRecord> {
        self.records
            .iter()
            .find(|record| record.kind == kind && record.key == key)
    }

    pub fn require(&self, kind: u32, key: u32) -> AssetResult<&SectionRecord> {
        self.get(kind, key)
            .ok_or(AssetError::MissingRequiredSection(kind, key))
    }

    pub fn find_all(&self, kind: u32) -> impl Iterator<Item = &SectionRecord> {
        self.records
            .iter()
            .filter(move |record| record.kind == kind)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct SectionBuild {
    pub kind: u32,
    pub key: u32,
    pub flags: u32,
    pub extra: u32,
    pub bytes: Bytes,
}
