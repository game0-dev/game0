use bytes::Bytes;
use file_core::{
    AssetError, AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader,
};

pub const SKELETON0_VERSION: u32 = 1;

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

    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
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
    pub async fn read<R>(reader: OffsetAssetReader<R>) -> AssetResult<Self>
    where
        R: AssetRead + Clone + Send + Sync,
    {
        let header = reader.read_at(0, 8).await?;
        let mut cursor = DecodeCursor::new(&header);
        let version = cursor.read_u32_le()?;
        if version != SKELETON0_VERSION {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        let bone_count = cursor.read_u32_le()? as u64;
        let len = 8_u64
            .checked_add(
                bone_count
                    .checked_mul(Skeleton0Bone::BYTE_SIZE as u64)
                    .ok_or(AssetError::OffsetOverflow)?,
            )
            .ok_or(AssetError::OffsetOverflow)?;
        Self::read_bytes(reader.read_at(0, len).await?)
    }

    pub fn read_bytes(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(&bytes);
        let version = cursor.read_u32_le()?;
        if version != SKELETON0_VERSION {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        let bone_count = cursor.read_u32_le()? as usize;
        let mut bones = Vec::with_capacity(bone_count);
        for _ in 0..bone_count {
            bones.push(Skeleton0Bone::read(&mut cursor)?);
        }
        if cursor.remaining() != 0 {
            return Err(AssetError::InvalidData("trailing skeleton0 bytes"));
        }
        Ok(Self { bones })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        out.write_u32_le(SKELETON0_VERSION);
        out.write_u32_le(u32::try_from(self.bones.len())?);
        for bone in &self.bones {
            bone.write(&mut out);
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

fn read_f32x3(cursor: &mut DecodeCursor<'_>) -> AssetResult<[f32; 3]> {
    Ok([
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
    ])
}

fn read_f32x16(cursor: &mut DecodeCursor<'_>) -> AssetResult<[f32; 16]> {
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
