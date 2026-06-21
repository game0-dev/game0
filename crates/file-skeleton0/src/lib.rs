use bytes::Bytes;
use file_core::{AssetError, AssetReader, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
struct Skeleton0Header {
    bone_count: u32,
    bone_offset: u64,
    bone_len: u64,
    file_len: u64,
}

impl Skeleton0Header {
    const VERSION: u32 = 1;
    const BYTE_SIZE: u64 = 8;

    async fn read<R>(reader: &R) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let mut cursor = DecodeCursor::from_reader(reader, Self::BYTE_SIZE).await?;
        Self::read_from_cursor(&mut cursor)
    }

    fn read_from_cursor(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        let version = cursor.read_u32_le()?;
        if version != Self::VERSION {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        Self::new(cursor.read_u32_le()?)
    }

    fn new(bone_count: u32) -> AssetResult<Self> {
        let bone_offset = Self::BYTE_SIZE;
        let bone_len = u64::from(bone_count)
            .checked_mul(Skeleton0Bone::BYTE_SIZE as u64)
            .ok_or(AssetError::OffsetOverflow)?;
        let file_len = bone_offset
            .checked_add(bone_len)
            .ok_or(AssetError::OffsetOverflow)?;
        Ok(Self {
            bone_count,
            bone_offset,
            bone_len,
            file_len,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(Self::VERSION);
        out.write_u32_le(self.bone_count);
    }
}

#[derive(Debug, Clone)]
pub struct Skeleton0Reader {
    pub bones: Vec<Skeleton0Bone>,
}

#[derive(Debug, Clone)]
pub struct Skeleton0Bone {
    pub parent_index: i32,
    pub flags: u32,
    pub key_bone_id: i16,
    pub pivot: [f32; 3],
    pub inverse_bind_matrix: [f32; 16],
    pub bind_matrix: [f32; 16],
    pub source_bone_index: u32,
}

impl Skeleton0Bone {
    pub const BYTE_SIZE: usize = 4 + 4 + 2 + 2 + 12 + 64 + 64 + 4;

    fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        let parent_index = cursor.read_i32_le()?;
        let flags = cursor.read_u32_le()?;
        let key_bone_id = cursor.read_u16_le()? as i16;
        let _reserved = cursor.read_u16_le()?;
        let pivot = read_f32x3(cursor)?;
        let inverse_bind_matrix = read_f32x16(cursor)?;
        let bind_matrix = read_f32x16(cursor)?;
        let source_bone_index = cursor.read_u32_le()?;
        Ok(Self {
            parent_index,
            flags,
            key_bone_id,
            pivot,
            inverse_bind_matrix,
            bind_matrix,
            source_bone_index,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_i32_le(self.parent_index);
        out.write_u32_le(self.flags);
        out.write_u16_le(self.key_bone_id as u16);
        out.write_u16_le(0);
        write_f32x3(out, self.pivot);
        write_f32x16(out, self.inverse_bind_matrix);
        write_f32x16(out, self.bind_matrix);
        out.write_u32_le(self.source_bone_index);
    }
}

impl Skeleton0Reader {
    pub async fn read<R>(reader: R) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let header = Skeleton0Header::read(&reader).await?;
        Self::read_bytes(reader.read_at(0, header.file_len).await?)
    }

    pub fn read_bytes(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(bytes);
        let header = Skeleton0Header::read_from_cursor(&mut cursor)?;
        cursor.seek(usize::try_from(header.bone_offset)?)?;
        let bone_count = usize::try_from(header.bone_count)?;
        let mut bones = Vec::with_capacity(bone_count);
        for _ in 0..bone_count {
            bones.push(Skeleton0Bone::read(&mut cursor)?);
        }
        let bone_end = header
            .bone_offset
            .checked_add(header.bone_len)
            .ok_or(AssetError::OffsetOverflow)?;
        if cursor.position() != usize::try_from(bone_end)? {
            return Err(AssetError::InvalidData("invalid skeleton0 bone table size"));
        }
        if cursor.remaining() != 0 {
            return Err(AssetError::InvalidData("trailing skeleton0 bytes"));
        }
        Ok(Self { bones })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        Skeleton0Header::new(u32::try_from(self.bones.len())?)?.write(&mut out);
        for bone in &self.bones {
            bone.write(&mut out);
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

fn read_f32x3(cursor: &mut DecodeCursor) -> AssetResult<[f32; 3]> {
    Ok([
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
    ])
}

fn read_f32x16(cursor: &mut DecodeCursor) -> AssetResult<[f32; 16]> {
    let mut value = [0.0; 16];
    for item in &mut value {
        *item = cursor.read_f32_le()?;
    }
    Ok(value)
}

fn write_f32x3(out: &mut EncodeBuffer, value: [f32; 3]) {
    for item in value {
        out.write_f32_le(item);
    }
}

fn write_f32x16(out: &mut EncodeBuffer, value: [f32; 16]) {
    for item in value {
        out.write_f32_le(item);
    }
}
