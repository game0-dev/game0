use bytes::Bytes;
use file_core::{
    AssetError, AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader,
};

pub const ANIM0_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct Anim0Reader {
    pub clips: Vec<Anim0AnimationClip>,
}

#[derive(Debug, Clone)]
pub struct Anim0AnimationClip {
    pub sequence_index: u32,
    pub animation_id: u16,
    pub sub_animation_id: u16,
    pub duration_ms: u32,
    pub flags: u32,
    pub bone_tracks: Vec<Anim0BoneAnimationTrack>,
}

#[derive(Debug, Clone)]
pub struct Anim0BoneAnimationTrack {
    pub bone_index: u32,
    pub translations: Vec<Anim0Vec3Key>,
    pub rotations: Vec<Anim0QuatKey>,
    pub scales: Vec<Anim0Vec3Key>,
}

#[derive(Debug, Clone, Copy)]
pub struct Anim0Vec3Key {
    pub time_ms: u32,
    pub value: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct Anim0QuatKey {
    pub time_ms: u32,
    pub value: [f32; 4],
}

impl Anim0Reader {
    pub async fn read<R>(reader: OffsetAssetReader<R>) -> AssetResult<Self>
    where
        R: AssetRead + Clone + Send + Sync,
    {
        let mut offset = 0;
        let bytes = read_chunk(&reader, &mut offset, 8).await?;
        let mut cursor = DecodeCursor::new(&bytes);
        let version = cursor.read_u32_le()?;
        if version != ANIM0_VERSION {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        let clip_count = cursor.read_u32_le()? as usize;
        let mut clips = Vec::with_capacity(clip_count);
        for _ in 0..clip_count {
            clips.push(read_animation_clip(&reader, &mut offset).await?);
        }
        Ok(Self { clips })
    }

    pub fn read_bytes(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(&bytes);
        let version = cursor.read_u32_le()?;
        if version != ANIM0_VERSION {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        let clip_count = cursor.read_u32_le()? as usize;
        let mut clips = Vec::with_capacity(clip_count);
        for _ in 0..clip_count {
            clips.push(Anim0AnimationClip::read(&mut cursor)?);
        }
        if cursor.remaining() != 0 {
            return Err(AssetError::InvalidData("trailing anim0 bytes"));
        }
        Ok(Self { clips })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        out.write_u32_le(ANIM0_VERSION);
        out.write_u32_le(u32::try_from(self.clips.len())?);
        for clip in &self.clips {
            clip.write(&mut out)?;
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

async fn read_animation_clip<R>(
    reader: &OffsetAssetReader<R>,
    offset: &mut u64,
) -> AssetResult<Anim0AnimationClip>
where
    R: AssetRead + Clone + Send + Sync,
{
    let bytes = read_chunk(reader, offset, 20).await?;
    let mut cursor = DecodeCursor::new(&bytes);
    let sequence_index = cursor.read_u32_le()?;
    let animation_id = cursor.read_u16_le()?;
    let sub_animation_id = cursor.read_u16_le()?;
    let duration_ms = cursor.read_u32_le()?;
    let flags = cursor.read_u32_le()?;
    let bone_track_count = cursor.read_u32_le()? as usize;
    let mut bone_tracks = Vec::with_capacity(bone_track_count);
    for _ in 0..bone_track_count {
        bone_tracks.push(read_bone_animation_track(reader, offset).await?);
    }
    Ok(Anim0AnimationClip {
        sequence_index,
        animation_id,
        sub_animation_id,
        duration_ms,
        flags,
        bone_tracks,
    })
}

async fn read_bone_animation_track<R>(
    reader: &OffsetAssetReader<R>,
    offset: &mut u64,
) -> AssetResult<Anim0BoneAnimationTrack>
where
    R: AssetRead + Clone + Send + Sync,
{
    let bytes = read_chunk(reader, offset, 4).await?;
    let mut cursor = DecodeCursor::new(&bytes);
    let bone_index = cursor.read_u32_le()?;
    let translations = read_vec3_keys_from_reader(reader, offset).await?;
    let rotations = read_quat_keys_from_reader(reader, offset).await?;
    let scales = read_vec3_keys_from_reader(reader, offset).await?;
    Ok(Anim0BoneAnimationTrack {
        bone_index,
        translations,
        rotations,
        scales,
    })
}

async fn read_vec3_keys_from_reader<R>(
    reader: &OffsetAssetReader<R>,
    offset: &mut u64,
) -> AssetResult<Vec<Anim0Vec3Key>>
where
    R: AssetRead + Clone + Send + Sync,
{
    let count = read_count(reader, offset).await?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes = read_chunk(reader, offset, 16).await?;
        let mut cursor = DecodeCursor::new(&bytes);
        keys.push(Anim0Vec3Key {
            time_ms: cursor.read_u32_le()?,
            value: [
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
            ],
        });
    }
    Ok(keys)
}

async fn read_quat_keys_from_reader<R>(
    reader: &OffsetAssetReader<R>,
    offset: &mut u64,
) -> AssetResult<Vec<Anim0QuatKey>>
where
    R: AssetRead + Clone + Send + Sync,
{
    let count = read_count(reader, offset).await?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes = read_chunk(reader, offset, 20).await?;
        let mut cursor = DecodeCursor::new(&bytes);
        keys.push(Anim0QuatKey {
            time_ms: cursor.read_u32_le()?,
            value: [
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
            ],
        });
    }
    Ok(keys)
}

async fn read_count<R>(reader: &OffsetAssetReader<R>, offset: &mut u64) -> AssetResult<usize>
where
    R: AssetRead + Clone + Send + Sync,
{
    let bytes = read_chunk(reader, offset, 4).await?;
    let mut cursor = DecodeCursor::new(&bytes);
    Ok(cursor.read_u32_le()? as usize)
}

async fn read_chunk<R>(
    reader: &OffsetAssetReader<R>,
    offset: &mut u64,
    len: u64,
) -> AssetResult<Bytes>
where
    R: AssetRead + Clone + Send + Sync,
{
    let bytes = reader.read_at(*offset, len).await?;
    *offset = offset.checked_add(len).ok_or(AssetError::OffsetOverflow)?;
    Ok(bytes)
}

impl Anim0AnimationClip {
    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        let sequence_index = cursor.read_u32_le()?;
        let animation_id = cursor.read_u16_le()?;
        let sub_animation_id = cursor.read_u16_le()?;
        let duration_ms = cursor.read_u32_le()?;
        let flags = cursor.read_u32_le()?;
        let bone_track_count = cursor.read_u32_le()? as usize;
        let mut bone_tracks = Vec::with_capacity(bone_track_count);
        for _ in 0..bone_track_count {
            bone_tracks.push(Anim0BoneAnimationTrack::read(cursor)?);
        }
        Ok(Self {
            sequence_index,
            animation_id,
            sub_animation_id,
            duration_ms,
            flags,
            bone_tracks,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) -> AssetResult<()> {
        out.write_u32_le(self.sequence_index);
        out.write_u16_le(self.animation_id);
        out.write_u16_le(self.sub_animation_id);
        out.write_u32_le(self.duration_ms);
        out.write_u32_le(self.flags);
        out.write_u32_le(u32::try_from(self.bone_tracks.len())?);
        for track in &self.bone_tracks {
            track.write(out)?;
        }
        Ok(())
    }
}

impl Anim0BoneAnimationTrack {
    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        let bone_index = cursor.read_u32_le()?;
        let translations = read_vec3_keys(cursor)?;
        let rotations = read_quat_keys(cursor)?;
        let scales = read_vec3_keys(cursor)?;
        Ok(Self {
            bone_index,
            translations,
            rotations,
            scales,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) -> AssetResult<()> {
        out.write_u32_le(self.bone_index);
        write_vec3_keys(out, &self.translations)?;
        write_quat_keys(out, &self.rotations)?;
        write_vec3_keys(out, &self.scales)?;
        Ok(())
    }
}

fn read_vec3_keys(cursor: &mut DecodeCursor<'_>) -> AssetResult<Vec<Anim0Vec3Key>> {
    let count = cursor.read_u32_le()? as usize;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        keys.push(Anim0Vec3Key {
            time_ms: cursor.read_u32_le()?,
            value: [
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
            ],
        });
    }
    Ok(keys)
}

fn read_quat_keys(cursor: &mut DecodeCursor<'_>) -> AssetResult<Vec<Anim0QuatKey>> {
    let count = cursor.read_u32_le()? as usize;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        keys.push(Anim0QuatKey {
            time_ms: cursor.read_u32_le()?,
            value: [
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
                cursor.read_f32_le()?,
            ],
        });
    }
    Ok(keys)
}

fn write_vec3_keys(out: &mut EncodeBuffer, keys: &[Anim0Vec3Key]) -> AssetResult<()> {
    out.write_u32_le(u32::try_from(keys.len())?);
    for key in keys {
        out.write_u32_le(key.time_ms);
        for item in key.value {
            out.write_f32_le(item);
        }
    }
    Ok(())
}

fn write_quat_keys(out: &mut EncodeBuffer, keys: &[Anim0QuatKey]) -> AssetResult<()> {
    out.write_u32_le(u32::try_from(keys.len())?);
    for key in keys {
        out.write_u32_le(key.time_ms);
        for item in key.value {
            out.write_f32_le(item);
        }
    }
    Ok(())
}
