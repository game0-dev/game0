use bytes::Bytes;

use crate::{AssetError, AssetId128, AssetReader, AssetResult};

pub struct DecodeCursor {
    bytes: Bytes,
    pos: usize,
}

impl DecodeCursor {
    pub fn new(bytes: Bytes) -> Self {
        Self { bytes, pos: 0 }
    }

    pub async fn from_reader<R>(reader: &R, len: u64) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        Self::from_reader_at(reader, 0, len).await
    }

    pub async fn from_reader_at<R>(reader: &R, offset: u64, len: u64) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let bytes = reader.read_at(offset, len).await?;
        Ok(Self::new(bytes))
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    pub fn seek(&mut self, pos: usize) -> AssetResult<()> {
        if pos > self.bytes.len() {
            return Err(AssetError::UnexpectedEof);
        }
        self.pos = pos;
        Ok(())
    }

    pub fn read_u8(&mut self) -> AssetResult<u8> {
        Ok(self.read_bytes(1)?[0])
    }

    pub fn read_u16_le(&mut self) -> AssetResult<u16> {
        Ok(u16::from_le_bytes(self.read_bytes(2)?.try_into().unwrap()))
    }

    pub fn read_u32_le(&mut self) -> AssetResult<u32> {
        Ok(u32::from_le_bytes(self.read_bytes(4)?.try_into().unwrap()))
    }

    pub fn read_i32_le(&mut self) -> AssetResult<i32> {
        Ok(i32::from_le_bytes(self.read_bytes(4)?.try_into().unwrap()))
    }

    pub fn read_u64_le(&mut self) -> AssetResult<u64> {
        Ok(u64::from_le_bytes(self.read_bytes(8)?.try_into().unwrap()))
    }

    pub fn read_f32_le(&mut self) -> AssetResult<f32> {
        Ok(f32::from_le_bytes(self.read_bytes(4)?.try_into().unwrap()))
    }

    pub fn read_f32x<const N: usize>(&mut self) -> AssetResult<[f32; N]> {
        let mut value = [0.0; N];
        for item in &mut value {
            *item = self.read_f32_le()?;
        }
        Ok(value)
    }

    pub fn read_f32x3(&mut self) -> AssetResult<[f32; 3]> {
        self.read_f32x()
    }

    pub fn read_bytes(&mut self, len: usize) -> AssetResult<&[u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(AssetError::OffsetOverflow)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(AssetError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn read_asset_id128(&mut self) -> AssetResult<AssetId128> {
        let mut id = [0; 16];
        id.copy_from_slice(self.read_bytes(16)?);
        Ok(AssetId128(id))
    }
}

pub fn decode_table<T>(
    bytes: Bytes,
    stride: usize,
    decode: fn(&mut DecodeCursor) -> AssetResult<T>,
) -> AssetResult<Vec<T>> {
    if stride == 0 || bytes.len() % stride != 0 {
        return Err(AssetError::InvalidData("invalid table size"));
    }
    let count = bytes.len() / stride;
    let mut cursor = DecodeCursor::new(bytes);
    let mut values = Vec::with_capacity(count);
    while cursor.remaining() > 0 {
        values.push(decode(&mut cursor)?);
    }
    Ok(values)
}

pub fn read_f32x3(cursor: &mut DecodeCursor) -> AssetResult<[f32; 3]> {
    cursor.read_f32x3()
}
