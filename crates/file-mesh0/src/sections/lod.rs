use bytes::Bytes;
use file_core::{
    align::{align_up, checked_range},
    read_f32x3, AssetError, AssetRead, AssetResult, DecodeCursor, EncodeBuffer, OffsetAssetReader,
};

pub mod index_format {
    pub const UINT16: u32 = 1;
    pub const UINT32: u32 = 2;
}

pub mod primitive_topology {
    pub const TRIANGLE_LIST: u32 = 1;
}

pub mod vertex_layout {
    pub const POSITION_NORMAL_UV0: u32 = 1;
    pub const POSITION_NORMAL_UV0_SKINNED: u32 = 2;
}

pub mod vertex_attribute {
    pub const POSITION: u32 = 1 << 0;
    pub const NORMAL: u32 = 1 << 1;
    pub const TANGENT: u32 = 1 << 2;
    pub const UV0: u32 = 1 << 3;
    pub const UV1: u32 = 1 << 4;
    pub const COLOR0: u32 = 1 << 5;
    pub const JOINTS0: u32 = 1 << 6;
    pub const WEIGHTS0: u32 = 1 << 7;
}

#[derive(Debug, Clone)]
pub struct Mesh0LodHeader {
    pub lod_level: u32,
    pub lod_flags: u32,
    pub screen_size: f32,
    pub max_distance: f32,
    pub primitive_topology: u32,
    pub vertex_layout_id: u32,
    pub vertex_attribute_mask: u32,
    pub vertex_stride: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub index_format: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounding_sphere_center: [f32; 3],
    pub bounding_sphere_radius: f32,
    pub submesh_count: u32,
    pub draw_batch_count: u32,
    pub joint_palette_count: u32,
    pub vertex_buffer_size: u32,
    pub index_buffer_size: u32,
}

impl Mesh0LodHeader {
    pub const BYTE_SIZE: usize = 104;

    fn read(bytes: Bytes) -> AssetResult<Self> {
        if bytes.len() != Self::BYTE_SIZE {
            return Err(AssetError::UnexpectedEof);
        }
        let mut cursor = DecodeCursor::new(&bytes);
        Ok(Self {
            lod_level: cursor.read_u32_le()?,
            lod_flags: cursor.read_u32_le()?,
            screen_size: cursor.read_f32_le()?,
            max_distance: cursor.read_f32_le()?,
            primitive_topology: cursor.read_u32_le()?,
            vertex_layout_id: cursor.read_u32_le()?,
            vertex_attribute_mask: cursor.read_u32_le()?,
            vertex_stride: cursor.read_u32_le()?,
            vertex_count: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            index_format: cursor.read_u32_le()?,
            bounds_min: read_f32x3(&mut cursor)?,
            bounds_max: read_f32x3(&mut cursor)?,
            bounding_sphere_center: read_f32x3(&mut cursor)?,
            bounding_sphere_radius: cursor.read_f32_le()?,
            submesh_count: cursor.read_u32_le()?,
            draw_batch_count: cursor.read_u32_le()?,
            joint_palette_count: cursor.read_u32_le()?,
            vertex_buffer_size: cursor.read_u32_le()?,
            index_buffer_size: cursor.read_u32_le()?,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.lod_level);
        out.write_u32_le(self.lod_flags);
        out.write_f32_le(self.screen_size);
        out.write_f32_le(self.max_distance);
        out.write_u32_le(self.primitive_topology);
        out.write_u32_le(self.vertex_layout_id);
        out.write_u32_le(self.vertex_attribute_mask);
        out.write_u32_le(self.vertex_stride);
        out.write_u32_le(self.vertex_count);
        out.write_u32_le(self.index_count);
        out.write_u32_le(self.index_format);
        out.write_f32x3(self.bounds_min);
        out.write_f32x3(self.bounds_max);
        out.write_f32x3(self.bounding_sphere_center);
        out.write_f32_le(self.bounding_sphere_radius);
        out.write_u32_le(self.submesh_count);
        out.write_u32_le(self.draw_batch_count);
        out.write_u32_le(self.joint_palette_count);
        out.write_u32_le(self.vertex_buffer_size);
        out.write_u32_le(self.index_buffer_size);
    }

    fn metadata_len(&self) -> AssetResult<usize> {
        let mut len = Self::BYTE_SIZE;
        len = add_array_len(len, self.submesh_count, Mesh0Submesh::BYTE_SIZE)?;
        len = add_array_len(len, self.draw_batch_count, Mesh0DrawBatch::BYTE_SIZE)?;
        add_array_len(
            len,
            self.joint_palette_count,
            Mesh0JointPaletteEntry::BYTE_SIZE,
        )
    }

    fn vertex_buffer_offset(&self) -> AssetResult<u64> {
        Ok(u64::try_from(align_up(self.metadata_len()?, 8)?)?)
    }

    fn index_buffer_offset(&self) -> AssetResult<u64> {
        self.vertex_buffer_offset()?
            .checked_add(u64::from(self.vertex_buffer_size))
            .ok_or(AssetError::OffsetOverflow)
    }

    fn validate_layout(&self, section_len: u32) -> AssetResult<()> {
        let required_len = self
            .index_buffer_offset()?
            .checked_add(u64::from(self.index_buffer_size))
            .ok_or(AssetError::OffsetOverflow)?;
        if required_len > u64::from(section_len) {
            return Err(AssetError::RangeOutOfBounds);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Mesh0Submesh {
    pub submesh_id: u32,
    pub flags: u32,
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
    pub material_slot: u32,
    pub joint_palette_start: u32,
    pub joint_palette_count: u32,
    pub max_bone_influence: u32,
    pub center: [f32; 3],
    pub sort_center: [f32; 3],
    pub bounding_radius: f32,
    pub source_submesh_id: u32,
    pub source_level: u32,
}

impl Mesh0Submesh {
    pub const BYTE_SIZE: usize = 76;

    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            submesh_id: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
            vertex_start: cursor.read_u32_le()?,
            vertex_count: cursor.read_u32_le()?,
            index_start: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            material_slot: cursor.read_u32_le()?,
            joint_palette_start: cursor.read_u32_le()?,
            joint_palette_count: cursor.read_u32_le()?,
            max_bone_influence: cursor.read_u32_le()?,
            center: read_f32x3(cursor)?,
            sort_center: read_f32x3(cursor)?,
            bounding_radius: cursor.read_f32_le()?,
            source_submesh_id: cursor.read_u32_le()?,
            source_level: cursor.read_u32_le()?,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.submesh_id);
        out.write_u32_le(self.flags);
        out.write_u32_le(self.vertex_start);
        out.write_u32_le(self.vertex_count);
        out.write_u32_le(self.index_start);
        out.write_u32_le(self.index_count);
        out.write_u32_le(self.material_slot);
        out.write_u32_le(self.joint_palette_start);
        out.write_u32_le(self.joint_palette_count);
        out.write_u32_le(self.max_bone_influence);
        out.write_f32x3(self.center);
        out.write_f32x3(self.sort_center);
        out.write_f32_le(self.bounding_radius);
        out.write_u32_le(self.source_submesh_id);
        out.write_u32_le(self.source_level);
    }
}

#[derive(Debug, Clone)]
pub struct Mesh0DrawBatch {
    pub batch_id: u32,
    pub flags: u32,
    pub submesh_index: u32,
    pub material_slot: u32,
    pub render_queue: u32,
    pub shader_hint: u32,
    pub priority: i32,
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
    pub source_skin_batch_index: u32,
    pub source_skin_section_index: u32,
    pub source_geoset_index: u32,
    pub source_material_index: u32,
    pub source_texture_combo_index: u32,
}

impl Mesh0DrawBatch {
    pub const BYTE_SIZE: usize = 64;

    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            batch_id: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
            submesh_index: cursor.read_u32_le()?,
            material_slot: cursor.read_u32_le()?,
            render_queue: cursor.read_u32_le()?,
            shader_hint: cursor.read_u32_le()?,
            priority: cursor.read_i32_le()?,
            vertex_start: cursor.read_u32_le()?,
            vertex_count: cursor.read_u32_le()?,
            index_start: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            source_skin_batch_index: cursor.read_u32_le()?,
            source_skin_section_index: cursor.read_u32_le()?,
            source_geoset_index: cursor.read_u32_le()?,
            source_material_index: cursor.read_u32_le()?,
            source_texture_combo_index: cursor.read_u32_le()?,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.batch_id);
        out.write_u32_le(self.flags);
        out.write_u32_le(self.submesh_index);
        out.write_u32_le(self.material_slot);
        out.write_u32_le(self.render_queue);
        out.write_u32_le(self.shader_hint);
        out.write_i32_le(self.priority);
        out.write_u32_le(self.vertex_start);
        out.write_u32_le(self.vertex_count);
        out.write_u32_le(self.index_start);
        out.write_u32_le(self.index_count);
        out.write_u32_le(self.source_skin_batch_index);
        out.write_u32_le(self.source_skin_section_index);
        out.write_u32_le(self.source_geoset_index);
        out.write_u32_le(self.source_material_index);
        out.write_u32_le(self.source_texture_combo_index);
    }
}

#[derive(Debug, Clone)]
pub struct Mesh0JointPaletteEntry {
    pub local_joint_index: u32,
    pub skeleton_joint_index: u32,
    pub source_bone_index: u32,
    pub flags: u32,
}

impl Mesh0JointPaletteEntry {
    pub const BYTE_SIZE: usize = 16;

    fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            local_joint_index: cursor.read_u32_le()?,
            skeleton_joint_index: cursor.read_u32_le()?,
            source_bone_index: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
        })
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.local_joint_index);
        out.write_u32_le(self.skeleton_joint_index);
        out.write_u32_le(self.source_bone_index);
        out.write_u32_le(self.flags);
    }
}

#[derive(Debug, Clone)]
pub struct LodSectionOwned {
    pub header: Mesh0LodHeader,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
    pub vertex_bytes: Bytes,
    pub index_bytes: Bytes,
}

impl LodSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        let mut header = self.header.clone();
        header.submesh_count = u32::try_from(self.submeshes.len())?;
        header.draw_batch_count = u32::try_from(self.draw_batches.len())?;
        header.joint_palette_count = u32::try_from(self.joint_palette.len())?;
        header.vertex_buffer_size = u32::try_from(self.vertex_bytes.len())?;
        header.index_buffer_size = u32::try_from(self.index_bytes.len())?;

        let mut out = EncodeBuffer::new();
        header.write(&mut out);
        for value in &self.submeshes {
            value.write(&mut out);
        }
        for value in &self.draw_batches {
            value.write(&mut out);
        }
        for value in &self.joint_palette {
            value.write(&mut out);
        }
        out.pad_to_align(8);
        out.write_bytes(&self.vertex_bytes);
        out.write_bytes(&self.index_bytes);
        Ok(Bytes::from(out.into_inner()))
    }

    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        let (header, submeshes, draw_batches, joint_palette) = read_lod_metadata(bytes.clone())?;
        let vertex_range = checked_range(
            bytes.len() as u64,
            header.vertex_buffer_offset()?,
            u64::from(header.vertex_buffer_size),
        )?;
        let index_range = checked_range(
            bytes.len() as u64,
            header.index_buffer_offset()?,
            u64::from(header.index_buffer_size),
        )?;
        Ok(Self {
            header,
            submeshes,
            draw_batches,
            joint_palette,
            vertex_bytes: bytes.slice(vertex_range),
            index_bytes: bytes.slice(index_range),
        })
    }

    pub(crate) fn validate(&self, material_slot_count: u32) -> AssetResult<()> {
        let header = &self.header;
        if header.vertex_stride == 0 {
            return Err(AssetError::InvalidData(
                "vertex_stride must be greater than zero",
            ));
        }
        if header.vertex_count == 0 || header.index_count == 0 {
            return Err(AssetError::InvalidData("empty LOD buffers are invalid"));
        }
        if header.primitive_topology != primitive_topology::TRIANGLE_LIST {
            return Err(AssetError::InvalidData("unsupported primitive topology"));
        }
        let index_size = match header.index_format {
            index_format::UINT16 => 2,
            index_format::UINT32 => 4,
            _ => return Err(AssetError::InvalidData("invalid index format")),
        };
        let expected_vertices = header
            .vertex_count
            .checked_mul(header.vertex_stride)
            .ok_or(AssetError::OffsetOverflow)?;
        let expected_indices = header
            .index_count
            .checked_mul(index_size)
            .ok_or(AssetError::OffsetOverflow)?;
        if self.vertex_bytes.len() != expected_vertices as usize {
            return Err(AssetError::InvalidData("invalid vertex buffer size"));
        }
        if self.index_bytes.len() != expected_indices as usize {
            return Err(AssetError::InvalidData("invalid index buffer size"));
        }
        if !valid_bounds(header.bounds_min, header.bounds_max) {
            return Err(AssetError::InvalidData("invalid LOD bounds"));
        }

        for submesh in &self.submeshes {
            checked_range_u32(
                submesh.vertex_start,
                submesh.vertex_count,
                header.vertex_count,
            )?;
            checked_range_u32(submesh.index_start, submesh.index_count, header.index_count)?;
            checked_range_u32(
                submesh.joint_palette_start,
                submesh.joint_palette_count,
                self.joint_palette.len() as u32,
            )?;
            if submesh.material_slot >= material_slot_count {
                return Err(AssetError::InvalidData(
                    "submesh material slot out of range",
                ));
            }
        }
        for batch in &self.draw_batches {
            checked_range_u32(batch.vertex_start, batch.vertex_count, header.vertex_count)?;
            checked_range_u32(batch.index_start, batch.index_count, header.index_count)?;
            if batch.submesh_index >= self.submeshes.len() as u32 {
                return Err(AssetError::InvalidData("draw batch submesh out of range"));
            }
            if batch.material_slot >= material_slot_count {
                return Err(AssetError::InvalidData(
                    "draw batch material slot out of range",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct LodSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    pub header: Mesh0LodHeader,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
}

impl<R> LodSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub(crate) async fn read(reader: OffsetAssetReader<R>, len: u32) -> AssetResult<Self> {
        let (header, submeshes, draw_batches, joint_palette) =
            read_lod_metadata_from_reader(&reader, len).await?;
        Ok(Self {
            reader,
            len,
            header,
            submeshes,
            draw_batches,
            joint_palette,
        })
    }

    pub async fn vertex_bytes(&self) -> AssetResult<Bytes> {
        let offset = self.header.vertex_buffer_offset()?;
        let size = u64::from(self.header.vertex_buffer_size);
        checked_range(u64::from(self.len), offset, size)?;
        self.reader.read_at(offset, size).await
    }

    pub async fn index_bytes(&self) -> AssetResult<Bytes> {
        let offset = self.header.index_buffer_offset()?;
        let size = u64::from(self.header.index_buffer_size);
        checked_range(u64::from(self.len), offset, size)?;
        self.reader.read_at(offset, size).await
    }

    pub async fn read_owned(&self) -> AssetResult<LodSectionOwned> {
        Ok(LodSectionOwned {
            header: self.header.clone(),
            submeshes: self.submeshes.clone(),
            draw_batches: self.draw_batches.clone(),
            joint_palette: self.joint_palette.clone(),
            vertex_bytes: self.vertex_bytes().await?,
            index_bytes: self.index_bytes().await?,
        })
    }
}

async fn read_lod_metadata_from_reader<R>(
    reader: &OffsetAssetReader<R>,
    len: u32,
) -> AssetResult<(
    Mesh0LodHeader,
    Vec<Mesh0Submesh>,
    Vec<Mesh0DrawBatch>,
    Vec<Mesh0JointPaletteEntry>,
)>
where
    R: AssetRead + Clone + Send + Sync,
{
    if len < Mesh0LodHeader::BYTE_SIZE as u32 {
        return Err(AssetError::UnexpectedEof);
    }
    let header_bytes = reader.read_at(0, Mesh0LodHeader::BYTE_SIZE as u64).await?;
    let header = Mesh0LodHeader::read(header_bytes)?;
    header.validate_layout(len)?;
    let metadata_len = u64::try_from(header.metadata_len()?)?;
    read_lod_metadata(reader.read_at(0, metadata_len).await?)
}

fn read_lod_metadata(
    bytes: Bytes,
) -> AssetResult<(
    Mesh0LodHeader,
    Vec<Mesh0Submesh>,
    Vec<Mesh0DrawBatch>,
    Vec<Mesh0JointPaletteEntry>,
)> {
    let header = Mesh0LodHeader::read(bytes.slice(0..Mesh0LodHeader::BYTE_SIZE))?;
    let metadata_len = header.metadata_len()?;
    if bytes.len() < metadata_len {
        return Err(AssetError::UnexpectedEof);
    }
    let mut cursor = DecodeCursor::new(&bytes[Mesh0LodHeader::BYTE_SIZE..metadata_len]);
    let submeshes = read_items(&mut cursor, header.submesh_count, Mesh0Submesh::read)?;
    let draw_batches = read_items(&mut cursor, header.draw_batch_count, Mesh0DrawBatch::read)?;
    let joint_palette = read_items(
        &mut cursor,
        header.joint_palette_count,
        Mesh0JointPaletteEntry::read,
    )?;
    Ok((header, submeshes, draw_batches, joint_palette))
}

fn checked_range_u32(start: u32, count: u32, len: u32) -> AssetResult<()> {
    if start.checked_add(count).filter(|end| *end <= len).is_none() {
        return Err(AssetError::RangeOutOfBounds);
    }
    Ok(())
}

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}

fn add_array_len(base: usize, count: u32, item_size: usize) -> AssetResult<usize> {
    usize::try_from(count)?
        .checked_mul(item_size)
        .and_then(|size| base.checked_add(size))
        .ok_or(AssetError::OffsetOverflow)
}

fn read_items<T>(
    cursor: &mut DecodeCursor<'_>,
    count: u32,
    decode: fn(&mut DecodeCursor<'_>) -> AssetResult<T>,
) -> AssetResult<Vec<T>> {
    let mut items = Vec::with_capacity(usize::try_from(count)?);
    for _ in 0..count {
        items.push(decode(cursor)?);
    }
    Ok(items)
}
