use bytes::Bytes;
use file_core::{AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
pub struct AttachmentRefsSection {
    pub refs: Vec<u64>,
}

impl AttachmentRefsSection {
    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(&bytes);
        let mut refs = Vec::with_capacity(bytes.len() / 8);
        while cursor.remaining() > 0 {
            refs.push(cursor.read_u64_le()?);
        }
        Ok(Self { refs })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        for item in &self.refs {
            out.write_u64_le(*item);
        }
        Ok(Bytes::from(out.into_inner()))
    }
}
