use bytes::Bytes;
use file_core::{AssetError, AssetReader, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
struct Skeleton0Header {
    bone_count: u32,
    key_bone_lookup_count: u32,
    bone_lookup_count: u32,
    bone_offset: u64,
    bone_len: u64,
    key_bone_lookup_offset: u64,
    key_bone_lookup_len: u64,
    bone_lookup_offset: u64,
    bone_lookup_len: u64,
    file_len: u64,
}

impl Skeleton0Header {
    const BYTE_SIZE: u64 = 12;

    async fn read<R>(reader: &R) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let mut cursor = DecodeCursor::from_reader(reader, Self::BYTE_SIZE).await?;
        Self::read_from_cursor(&mut cursor)
    }

    fn read_from_cursor(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        Self::new(
            cursor.read_u32_le()?,
            cursor.read_u32_le()?,
            cursor.read_u32_le()?,
        )
    }

    fn new(
        bone_count: u32,
        key_bone_lookup_count: u32,
        bone_lookup_count: u32,
    ) -> AssetResult<Self> {
        let bone_offset = Self::BYTE_SIZE;
        let bone_len = u64::from(bone_count)
            .checked_mul(Skeleton0Bone::BYTE_SIZE as u64)
            .ok_or(AssetError::OffsetOverflow)?;
        let key_bone_lookup_offset = bone_offset
            .checked_add(bone_len)
            .ok_or(AssetError::OffsetOverflow)?;
        let key_bone_lookup_len = u64::from(key_bone_lookup_count)
            .checked_mul(2)
            .ok_or(AssetError::OffsetOverflow)?;
        let bone_lookup_offset = key_bone_lookup_offset
            .checked_add(key_bone_lookup_len)
            .ok_or(AssetError::OffsetOverflow)?;
        let bone_lookup_len = u64::from(bone_lookup_count)
            .checked_mul(2)
            .ok_or(AssetError::OffsetOverflow)?;
        let file_len = bone_lookup_offset
            .checked_add(bone_lookup_len)
            .ok_or(AssetError::OffsetOverflow)?;
        Ok(Self {
            bone_count,
            key_bone_lookup_count,
            bone_lookup_count,
            bone_offset,
            bone_len,
            key_bone_lookup_offset,
            key_bone_lookup_len,
            bone_lookup_offset,
            bone_lookup_len,
            file_len,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.bone_count);
        out.write_u32_le(self.key_bone_lookup_count);
        out.write_u32_le(self.bone_lookup_count);
    }
}

#[derive(Debug, Clone)]
pub struct Skeleton0Reader {
    pub bones: Vec<Skeleton0Bone>,
    pub key_bone_lookup: Vec<i16>,
    pub bone_lookup_table: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct Skeleton0Bone {
    pub parent_index: i32,
    pub flags: u32,
    pub key_bone_id: i16,
    pub submesh_id: u16,
    pub bone_name_crc: Option<u32>,
    pub unknown: [u16; 2],
    pub pivot: [f32; 3],
    pub inverse_bind_matrix: [f32; 16],
    pub bind_matrix: [f32; 16],
    pub source_bone_index: u32,
}

impl Skeleton0Bone {
    const BONE_NAME_CRC_NONE: u32 = u32::MAX;
    pub const BYTE_SIZE: usize = 4 + 4 + 2 + 2 + 4 + 2 + 2 + 12 + 64 + 64 + 4;

    fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        let parent_index = cursor.read_i32_le()?;
        let flags = cursor.read_u32_le()?;
        let key_bone_id = cursor.read_u16_le()? as i16;
        let submesh_id = cursor.read_u16_le()?;
        let bone_name_crc = match cursor.read_u32_le()? {
            Self::BONE_NAME_CRC_NONE => None,
            value => Some(value),
        };
        let unknown = [cursor.read_u16_le()?, cursor.read_u16_le()?];
        let pivot = cursor.read_f32x3()?;
        let inverse_bind_matrix = cursor.read_f32x()?;
        let bind_matrix = cursor.read_f32x()?;
        let source_bone_index = cursor.read_u32_le()?;
        Ok(Self {
            parent_index,
            flags,
            key_bone_id,
            submesh_id,
            bone_name_crc,
            unknown,
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
        out.write_u16_le(self.submesh_id);
        out.write_u32_le(self.bone_name_crc.unwrap_or(Self::BONE_NAME_CRC_NONE));
        out.write_u16_le(self.unknown[0]);
        out.write_u16_le(self.unknown[1]);
        out.write_f32x3(self.pivot);
        out.write_f32x(self.inverse_bind_matrix);
        out.write_f32x(self.bind_matrix);
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
        cursor.seek(usize::try_from(header.key_bone_lookup_offset)?)?;
        let key_bone_lookup = read_i16_table(&mut cursor, header.key_bone_lookup_count)?;
        let key_bone_lookup_end = header
            .key_bone_lookup_offset
            .checked_add(header.key_bone_lookup_len)
            .ok_or(AssetError::OffsetOverflow)?;
        if cursor.position() != usize::try_from(key_bone_lookup_end)? {
            return Err(AssetError::InvalidData(
                "invalid skeleton0 key bone lookup table size",
            ));
        }
        cursor.seek(usize::try_from(header.bone_lookup_offset)?)?;
        let bone_lookup_table = read_u16_table(&mut cursor, header.bone_lookup_count)?;
        let bone_lookup_end = header
            .bone_lookup_offset
            .checked_add(header.bone_lookup_len)
            .ok_or(AssetError::OffsetOverflow)?;
        if cursor.position() != usize::try_from(bone_lookup_end)? {
            return Err(AssetError::InvalidData(
                "invalid skeleton0 bone lookup table size",
            ));
        }
        if cursor.remaining() != 0 {
            return Err(AssetError::InvalidData("trailing skeleton0 bytes"));
        }
        Ok(Self {
            bones,
            key_bone_lookup,
            bone_lookup_table,
        })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        Skeleton0Header::new(
            u32::try_from(self.bones.len())?,
            u32::try_from(self.key_bone_lookup.len())?,
            u32::try_from(self.bone_lookup_table.len())?,
        )?
        .write(&mut out);
        for bone in &self.bones {
            bone.write(&mut out);
        }
        for key_bone in &self.key_bone_lookup {
            out.write_u16_le(*key_bone as u16);
        }
        for bone in &self.bone_lookup_table {
            out.write_u16_le(*bone);
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

fn read_i16_table(cursor: &mut DecodeCursor, count: u32) -> AssetResult<Vec<i16>> {
    let count = usize::try_from(count)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(cursor.read_u16_le()? as i16);
    }
    Ok(values)
}

fn read_u16_table(cursor: &mut DecodeCursor, count: u32) -> AssetResult<Vec<u16>> {
    let count = usize::try_from(count)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(cursor.read_u16_le()?);
    }
    Ok(values)
}
