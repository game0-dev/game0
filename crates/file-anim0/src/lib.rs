use bytes::Bytes;
use file_core::{AssetError, AssetReader, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
struct Anim0Header {
    clip_count: u32,
    clip_offset: u64,
}

impl Anim0Header {
    const BYTE_SIZE: u64 = 4;

    fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        Ok(Self {
            clip_count: cursor.read_u32_le()?,
            clip_offset: Self::BYTE_SIZE,
        })
    }

    fn new(clip_count: u32) -> Self {
        Self {
            clip_count,
            clip_offset: Self::BYTE_SIZE,
        }
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.clip_count);
    }
}

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
    pub movement_speed: f32,
    pub frequency: i16,
    pub replay_range_ms: Option<[u32; 2]>,
    pub bounds: Option<Anim0Bounds>,
    pub next_animation: Option<i16>,
    pub aliasing: Option<u16>,
    pub bone_tracks: Vec<Anim0BoneAnimationTrack>,
}

#[derive(Debug, Clone)]
pub struct Anim0BoneAnimationTrack {
    pub bone_index: u32,
    pub translation_info: Anim0TrackInfo,
    pub translations: Vec<Anim0Vec3Key>,
    pub rotation_info: Anim0TrackInfo,
    pub rotations: Vec<Anim0QuatKey>,
    pub scale_info: Anim0TrackInfo,
    pub scales: Vec<Anim0Vec3Key>,
}

#[derive(Debug, Clone, Copy)]
pub struct Anim0Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anim0Interpolation {
    None = 0,
    Linear = 1,
    Bezier = 2,
    Hermite = 3,
}

impl Default for Anim0Interpolation {
    fn default() -> Self {
        Self::None
    }
}

impl Anim0Interpolation {
    fn from_u16(value: u16) -> Self {
        match value {
            1 => Self::Linear,
            2 => Self::Bezier,
            3 => Self::Hermite,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Anim0TrackInfo {
    pub interpolation: Anim0Interpolation,
    pub global_sequence: i16,
    pub source_timestamp_count: u32,
    pub source_value_count: u32,
    pub ranges: Vec<Anim0TrackRange>,
}

impl Default for Anim0TrackInfo {
    fn default() -> Self {
        Self {
            interpolation: Anim0Interpolation::None,
            global_sequence: -1,
            source_timestamp_count: 0,
            source_value_count: 0,
            ranges: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Anim0TrackRange {
    pub start_ms: u32,
    pub end_ms: u32,
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
    pub async fn read<R>(reader: R) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let mut offset = 0;
        let bytes = read_chunk(&reader, &mut offset, Anim0Header::BYTE_SIZE).await?;
        let mut cursor = DecodeCursor::new(bytes);
        let header = Anim0Header::read(&mut cursor)?;
        offset = header.clip_offset;
        let clip_count = usize::try_from(header.clip_count)?;
        let mut clips = Vec::with_capacity(clip_count);
        for _ in 0..clip_count {
            clips.push(read_animation_clip(&reader, &mut offset).await?);
        }
        Ok(Self { clips })
    }

    pub fn read_bytes(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(bytes);
        let header = Anim0Header::read(&mut cursor)?;
        cursor.seek(usize::try_from(header.clip_offset)?)?;
        let clip_count = usize::try_from(header.clip_count)?;
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
        Anim0Header::new(u32::try_from(self.clips.len())?).write(&mut out);
        for clip in &self.clips {
            clip.write(&mut out)?;
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

async fn read_animation_clip<R>(reader: &R, offset: &mut u64) -> AssetResult<Anim0AnimationClip>
where
    R: AssetReader,
{
    let bytes = read_chunk(reader, offset, 20).await?;
    let mut cursor = DecodeCursor::new(bytes);
    let sequence_index = cursor.read_u32_le()?;
    let animation_id = cursor.read_u16_le()?;
    let sub_animation_id = cursor.read_u16_le()?;
    let duration_ms = cursor.read_u32_le()?;
    let flags = cursor.read_u32_le()?;
    let bone_track_count = cursor.read_u32_le()? as usize;
    let bytes = read_chunk(reader, offset, Anim0ClipMetadata::BYTE_SIZE).await?;
    let mut cursor = DecodeCursor::new(bytes);
    let metadata = Anim0ClipMetadata::read(&mut cursor)?;
    read_animation_clip_tail(
        reader,
        offset,
        sequence_index,
        animation_id,
        sub_animation_id,
        duration_ms,
        flags,
        bone_track_count,
        metadata,
    )
    .await
}

async fn read_animation_clip_tail<R>(
    reader: &R,
    offset: &mut u64,
    sequence_index: u32,
    animation_id: u16,
    sub_animation_id: u16,
    duration_ms: u32,
    flags: u32,
    bone_track_count: usize,
    metadata: Anim0ClipMetadata,
) -> AssetResult<Anim0AnimationClip>
where
    R: AssetReader,
{
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
        movement_speed: metadata.movement_speed,
        frequency: metadata.frequency,
        replay_range_ms: metadata.replay_range_ms,
        bounds: metadata.bounds,
        next_animation: metadata.next_animation,
        aliasing: metadata.aliasing,
        bone_tracks,
    })
}

async fn read_bone_animation_track<R>(
    reader: &R,
    offset: &mut u64,
) -> AssetResult<Anim0BoneAnimationTrack>
where
    R: AssetReader,
{
    let bytes = read_chunk(reader, offset, 4).await?;
    let mut cursor = DecodeCursor::new(bytes);
    let bone_index = cursor.read_u32_le()?;
    let translation_info = read_track_info_from_reader(reader, offset).await?;
    let translations = read_vec3_keys_from_reader(reader, offset).await?;
    let rotation_info = read_track_info_from_reader(reader, offset).await?;
    let rotations = read_quat_keys_from_reader(reader, offset).await?;
    let scale_info = read_track_info_from_reader(reader, offset).await?;
    let scales = read_vec3_keys_from_reader(reader, offset).await?;
    Ok(Anim0BoneAnimationTrack {
        bone_index,
        translation_info,
        translations,
        rotation_info,
        rotations,
        scale_info,
        scales,
    })
}

async fn read_track_info_from_reader<R>(reader: &R, offset: &mut u64) -> AssetResult<Anim0TrackInfo>
where
    R: AssetReader,
{
    let bytes = read_chunk(reader, offset, 16).await?;
    let mut cursor = DecodeCursor::new(bytes);
    let interpolation = Anim0Interpolation::from_u16(cursor.read_u16_le()?);
    let global_sequence = cursor.read_u16_le()? as i16;
    let source_timestamp_count = cursor.read_u32_le()?;
    let source_value_count = cursor.read_u32_le()?;
    let range_count = cursor.read_u32_le()? as usize;
    let mut ranges = Vec::with_capacity(range_count);
    for _ in 0..range_count {
        let bytes = read_chunk(reader, offset, 8).await?;
        let mut cursor = DecodeCursor::new(bytes);
        ranges.push(Anim0TrackRange {
            start_ms: cursor.read_u32_le()?,
            end_ms: cursor.read_u32_le()?,
        });
    }
    Ok(Anim0TrackInfo {
        interpolation,
        global_sequence,
        source_timestamp_count,
        source_value_count,
        ranges,
    })
}

async fn read_vec3_keys_from_reader<R>(
    reader: &R,
    offset: &mut u64,
) -> AssetResult<Vec<Anim0Vec3Key>>
where
    R: AssetReader,
{
    let count = read_count(reader, offset).await?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes = read_chunk(reader, offset, 16).await?;
        let mut cursor = DecodeCursor::new(bytes);
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
    reader: &R,
    offset: &mut u64,
) -> AssetResult<Vec<Anim0QuatKey>>
where
    R: AssetReader,
{
    let count = read_count(reader, offset).await?;
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes = read_chunk(reader, offset, 20).await?;
        let mut cursor = DecodeCursor::new(bytes);
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

async fn read_count<R>(reader: &R, offset: &mut u64) -> AssetResult<usize>
where
    R: AssetReader,
{
    let bytes = read_chunk(reader, offset, 4).await?;
    let mut cursor = DecodeCursor::new(bytes);
    Ok(cursor.read_u32_le()? as usize)
}

async fn read_chunk<R>(reader: &R, offset: &mut u64, len: u64) -> AssetResult<Bytes>
where
    R: AssetReader,
{
    let bytes = reader.read_at(*offset, len).await?;
    *offset = offset.checked_add(len).ok_or(AssetError::OffsetOverflow)?;
    Ok(bytes)
}

impl Anim0AnimationClip {
    fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        let sequence_index = cursor.read_u32_le()?;
        let animation_id = cursor.read_u16_le()?;
        let sub_animation_id = cursor.read_u16_le()?;
        let duration_ms = cursor.read_u32_le()?;
        let flags = cursor.read_u32_le()?;
        let bone_track_count = cursor.read_u32_le()? as usize;
        let metadata = Anim0ClipMetadata::read(cursor)?;
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
            movement_speed: metadata.movement_speed,
            frequency: metadata.frequency,
            replay_range_ms: metadata.replay_range_ms,
            bounds: metadata.bounds,
            next_animation: metadata.next_animation,
            aliasing: metadata.aliasing,
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
        Anim0ClipMetadata::from_clip(self).write(out);
        for track in &self.bone_tracks {
            track.write(out)?;
        }
        Ok(())
    }
}

impl Anim0BoneAnimationTrack {
    fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        let bone_index = cursor.read_u32_le()?;
        let translation_info = read_track_info(cursor)?;
        let translations = read_vec3_keys(cursor)?;
        let rotation_info = read_track_info(cursor)?;
        let rotations = read_quat_keys(cursor)?;
        let scale_info = read_track_info(cursor)?;
        let scales = read_vec3_keys(cursor)?;
        Ok(Self {
            bone_index,
            translation_info,
            translations,
            rotation_info,
            rotations,
            scale_info,
            scales,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) -> AssetResult<()> {
        out.write_u32_le(self.bone_index);
        write_track_info(out, &self.translation_info)?;
        write_vec3_keys(out, &self.translations)?;
        write_track_info(out, &self.rotation_info)?;
        write_quat_keys(out, &self.rotations)?;
        write_track_info(out, &self.scale_info)?;
        write_vec3_keys(out, &self.scales)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Anim0ClipMetadata {
    movement_speed: f32,
    frequency: i16,
    replay_range_ms: Option<[u32; 2]>,
    bounds: Option<Anim0Bounds>,
    next_animation: Option<i16>,
    aliasing: Option<u16>,
}

impl Default for Anim0ClipMetadata {
    fn default() -> Self {
        Self {
            movement_speed: 0.0,
            frequency: 0,
            replay_range_ms: None,
            bounds: None,
            next_animation: None,
            aliasing: None,
        }
    }
}

impl Anim0ClipMetadata {
    const HAS_REPLAY_RANGE: u32 = 1 << 0;
    const HAS_BOUNDS: u32 = 1 << 1;
    const HAS_NEXT_ANIMATION: u32 = 1 << 2;
    const HAS_ALIASING: u32 = 1 << 3;
    const BYTE_SIZE: u64 = 52;

    fn from_clip(clip: &Anim0AnimationClip) -> Self {
        Self {
            movement_speed: clip.movement_speed,
            frequency: clip.frequency,
            replay_range_ms: clip.replay_range_ms,
            bounds: clip.bounds,
            next_animation: clip.next_animation,
            aliasing: clip.aliasing,
        }
    }

    fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        let flags = cursor.read_u32_le()?;
        let movement_speed = cursor.read_f32_le()?;
        let frequency = cursor.read_u16_le()? as i16;
        let next_animation_value = cursor.read_u16_le()? as i16;
        let aliasing_value = cursor.read_u16_le()?;
        let _reserved = cursor.read_u16_le()?;
        let replay_start = cursor.read_u32_le()?;
        let replay_end = cursor.read_u32_le()?;
        let bounds_min = cursor.read_f32x3()?;
        let bounds_max = cursor.read_f32x3()?;
        let bounds_radius = cursor.read_f32_le()?;
        Ok(Self {
            movement_speed,
            frequency,
            replay_range_ms: (flags & Self::HAS_REPLAY_RANGE != 0)
                .then_some([replay_start, replay_end]),
            bounds: (flags & Self::HAS_BOUNDS != 0).then_some(Anim0Bounds {
                min: bounds_min,
                max: bounds_max,
                radius: bounds_radius,
            }),
            next_animation: (flags & Self::HAS_NEXT_ANIMATION != 0).then_some(next_animation_value),
            aliasing: (flags & Self::HAS_ALIASING != 0).then_some(aliasing_value),
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        let mut flags = 0;
        if self.replay_range_ms.is_some() {
            flags |= Self::HAS_REPLAY_RANGE;
        }
        if self.bounds.is_some() {
            flags |= Self::HAS_BOUNDS;
        }
        if self.next_animation.is_some() {
            flags |= Self::HAS_NEXT_ANIMATION;
        }
        if self.aliasing.is_some() {
            flags |= Self::HAS_ALIASING;
        }
        out.write_u32_le(flags);
        out.write_f32_le(self.movement_speed);
        out.write_u16_le(self.frequency as u16);
        out.write_u16_le(self.next_animation.unwrap_or(-1) as u16);
        out.write_u16_le(self.aliasing.unwrap_or(0));
        out.write_u16_le(0);
        let replay = self.replay_range_ms.unwrap_or([0, 0]);
        out.write_u32_le(replay[0]);
        out.write_u32_le(replay[1]);
        let bounds = self.bounds.unwrap_or(Anim0Bounds {
            min: [0.0; 3],
            max: [0.0; 3],
            radius: 0.0,
        });
        out.write_f32x3(bounds.min);
        out.write_f32x3(bounds.max);
        out.write_f32_le(bounds.radius);
    }
}

fn read_track_info(cursor: &mut DecodeCursor) -> AssetResult<Anim0TrackInfo> {
    let interpolation = Anim0Interpolation::from_u16(cursor.read_u16_le()?);
    let global_sequence = cursor.read_u16_le()? as i16;
    let source_timestamp_count = cursor.read_u32_le()?;
    let source_value_count = cursor.read_u32_le()?;
    let range_count = cursor.read_u32_le()? as usize;
    let mut ranges = Vec::with_capacity(range_count);
    for _ in 0..range_count {
        ranges.push(Anim0TrackRange {
            start_ms: cursor.read_u32_le()?,
            end_ms: cursor.read_u32_le()?,
        });
    }
    Ok(Anim0TrackInfo {
        interpolation,
        global_sequence,
        source_timestamp_count,
        source_value_count,
        ranges,
    })
}

fn write_track_info(out: &mut EncodeBuffer, info: &Anim0TrackInfo) -> AssetResult<()> {
    out.write_u16_le(info.interpolation as u16);
    out.write_u16_le(info.global_sequence as u16);
    out.write_u32_le(info.source_timestamp_count);
    out.write_u32_le(info.source_value_count);
    out.write_u32_le(u32::try_from(info.ranges.len())?);
    for range in &info.ranges {
        out.write_u32_le(range.start_ms);
        out.write_u32_le(range.end_ms);
    }
    Ok(())
}

fn read_vec3_keys(cursor: &mut DecodeCursor) -> AssetResult<Vec<Anim0Vec3Key>> {
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

fn read_quat_keys(cursor: &mut DecodeCursor) -> AssetResult<Vec<Anim0QuatKey>> {
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
