use crate::{AssetError, AssetId128, AssetResult};

pub struct DecodeCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> DecodeCursor<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
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

    pub fn read_bytes(&mut self, len: usize) -> AssetResult<&'a [u8]> {
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
    bytes: &[u8],
    stride: usize,
    decode: fn(&mut DecodeCursor<'_>) -> AssetResult<T>,
) -> AssetResult<Vec<T>> {
    if stride == 0 || bytes.len() % stride != 0 {
        return Err(AssetError::InvalidData("invalid table size"));
    }
    let mut cursor = DecodeCursor::new(bytes);
    let mut values = Vec::with_capacity(bytes.len() / stride);
    while cursor.remaining() > 0 {
        values.push(decode(&mut cursor)?);
    }
    Ok(values)
}

pub fn read_f32x3(cursor: &mut DecodeCursor<'_>) -> AssetResult<[f32; 3]> {
    Ok([
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
    ])
}
