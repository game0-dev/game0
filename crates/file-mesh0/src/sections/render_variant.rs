use bytes::Bytes;
use file_core::{
    align::{align_up, checked_range},
    AssetError, AssetRead, AssetResult, DecodeCursor, EncodeBuffer,
};

use super::{primitive_topology, MeshInfoHeader};

pub const NO_LOD_LEVEL: u32 = u32::MAX;

pub mod index_format {
    pub const UINT16: u32 = 1;
    pub const UINT32: u32 = 2;
}

#[derive(Debug, Clone)]
pub struct RenderVariantHeader {
    pub render_variant_index: u32,
    pub render_variant_flags: u32,
    pub lod_level: u32,
    pub screen_size: f32,
    pub max_distance: f32,
    pub primitive_topology: u32,
    pub index_count: u32,
    pub index_format: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounding_sphere_center: [f32; 3],
    pub bounding_sphere_radius: f32,
    pub submesh_count: u32,
    pub draw_batch_count: u32,
    pub joint_palette_count: u32,
    pub index_buffer_size: u32,
}

impl RenderVariantHeader {
    pub const BYTE_SIZE: usize = 88;

    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        if bytes.len() < Self::BYTE_SIZE {
            return Err(AssetError::UnexpectedEof);
        }
        let mut cursor = DecodeCursor::new(&bytes[..Self::BYTE_SIZE]);
        Ok(Self {
            render_variant_index: cursor.read_u32_le()?,
            render_variant_flags: cursor.read_u32_le()?,
            lod_level: cursor.read_u32_le()?,
            screen_size: cursor.read_f32_le()?,
            max_distance: cursor.read_f32_le()?,
            primitive_topology: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            index_format: cursor.read_u32_le()?,
            bounds_min: read_f32x3(&mut cursor)?,
            bounds_max: read_f32x3(&mut cursor)?,
            bounding_sphere_center: read_f32x3(&mut cursor)?,
            bounding_sphere_radius: cursor.read_f32_le()?,
            submesh_count: cursor.read_u32_le()?,
            draw_batch_count: cursor.read_u32_le()?,
            joint_palette_count: cursor.read_u32_le()?,
            index_buffer_size: cursor.read_u32_le()?,
        })
    }

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.render_variant_index);
        out.write_u32_le(self.render_variant_flags);
        out.write_u32_le(self.lod_level);
        out.write_f32_le(self.screen_size);
        out.write_f32_le(self.max_distance);
        out.write_u32_le(self.primitive_topology);
        out.write_u32_le(self.index_count);
        out.write_u32_le(self.index_format);
        write_f32x3(out, self.bounds_min);
        write_f32x3(out, self.bounds_max);
        write_f32x3(out, self.bounding_sphere_center);
        out.write_f32_le(self.bounding_sphere_radius);
        out.write_u32_le(self.submesh_count);
        out.write_u32_le(self.draw_batch_count);
        out.write_u32_le(self.joint_palette_count);
        out.write_u32_le(self.index_buffer_size);
    }

    pub fn metadata_len(&self) -> AssetResult<usize> {
        let mut len = Self::BYTE_SIZE;
        len = add_array_len(len, self.submesh_count, Mesh0Submesh::BYTE_SIZE)?;
        len = add_array_len(len, self.draw_batch_count, Mesh0DrawBatch::BYTE_SIZE)?;
        add_array_len(
            len,
            self.joint_palette_count,
            Mesh0JointPaletteEntry::BYTE_SIZE,
        )
    }

    pub fn index_buffer_offset(&self) -> AssetResult<u64> {
        Ok(u64::try_from(align_up(self.metadata_len()?, 8)?)?)
    }

    pub fn validate_layout(&self, section_len: u32) -> AssetResult<()> {
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
    pub const BYTE_SIZE: usize = 68;

    pub fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            submesh_id: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
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

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.submesh_id);
        out.write_u32_le(self.flags);
        out.write_u32_le(self.index_start);
        out.write_u32_le(self.index_count);
        out.write_u32_le(self.material_slot);
        out.write_u32_le(self.joint_palette_start);
        out.write_u32_le(self.joint_palette_count);
        out.write_u32_le(self.max_bone_influence);
        write_f32x3(out, self.center);
        write_f32x3(out, self.sort_center);
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
    pub index_start: u32,
    pub index_count: u32,
    pub source_skin_batch_index: u32,
    pub source_skin_section_index: u32,
    pub source_geoset_index: u32,
    pub source_material_index: u32,
    pub source_texture_combo_index: u32,
}

impl Mesh0DrawBatch {
    pub const BYTE_SIZE: usize = 56;

    pub fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            batch_id: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
            submesh_index: cursor.read_u32_le()?,
            material_slot: cursor.read_u32_le()?,
            render_queue: cursor.read_u32_le()?,
            shader_hint: cursor.read_u32_le()?,
            priority: cursor.read_i32_le()?,
            index_start: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            source_skin_batch_index: cursor.read_u32_le()?,
            source_skin_section_index: cursor.read_u32_le()?,
            source_geoset_index: cursor.read_u32_le()?,
            source_material_index: cursor.read_u32_le()?,
            source_texture_combo_index: cursor.read_u32_le()?,
        })
    }

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.batch_id);
        out.write_u32_le(self.flags);
        out.write_u32_le(self.submesh_index);
        out.write_u32_le(self.material_slot);
        out.write_u32_le(self.render_queue);
        out.write_u32_le(self.shader_hint);
        out.write_i32_le(self.priority);
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

    pub fn read(cursor: &mut DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Self {
            local_joint_index: cursor.read_u32_le()?,
            skeleton_joint_index: cursor.read_u32_le()?,
            source_bone_index: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
        })
    }

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.local_joint_index);
        out.write_u32_le(self.skeleton_joint_index);
        out.write_u32_le(self.source_bone_index);
        out.write_u32_le(self.flags);
    }
}

#[derive(Clone)]
pub struct RenderVariantReader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: R,
    offset: u64,
    len: u32,
    pub header: RenderVariantHeader,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
}

impl<R> RenderVariantReader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn read(reader: R, offset: u64, len: u32) -> AssetResult<Self> {
        if len < RenderVariantHeader::BYTE_SIZE as u32 {
            return Err(AssetError::UnexpectedEof);
        }
        let header = RenderVariantHeader::read(
            reader
                .read_at(offset, RenderVariantHeader::BYTE_SIZE as u64)
                .await?,
        )?;
        header.validate_layout(len)?;

        let metadata_len = u64::try_from(header.metadata_len()?)?;
        let metadata = reader.read_at(offset, metadata_len).await?;
        let mut cursor =
            DecodeCursor::new(&metadata[RenderVariantHeader::BYTE_SIZE..metadata_len as usize]);
        let submeshes = read_items(&mut cursor, header.submesh_count, Mesh0Submesh::read)?;
        let draw_batches = read_items(&mut cursor, header.draw_batch_count, Mesh0DrawBatch::read)?;
        let joint_palette = read_items(
            &mut cursor,
            header.joint_palette_count,
            Mesh0JointPaletteEntry::read,
        )?;

        Ok(Self {
            reader,
            offset,
            len,
            header,
            submeshes,
            draw_batches,
            joint_palette,
        })
    }

    pub async fn index_bytes(&self) -> AssetResult<Bytes> {
        let offset = self.header.index_buffer_offset()?;
        let size = u64::from(self.header.index_buffer_size);
        checked_range(u64::from(self.len), offset, size)?;
        let absolute = self
            .offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        self.reader.read_at(absolute, size).await
    }

    pub async fn read_builder(&self) -> AssetResult<RenderVariantBuilder> {
        Ok(RenderVariantBuilder {
            header: self.header.clone(),
            submeshes: self.submeshes.clone(),
            draw_batches: self.draw_batches.clone(),
            joint_palette: self.joint_palette.clone(),
            index_bytes: self.index_bytes().await?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RenderVariantBuilder {
    pub header: RenderVariantHeader,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
    pub index_bytes: Bytes,
}

impl RenderVariantBuilder {
    pub fn write(&self) -> AssetResult<Bytes> {
        let mut header = self.header.clone();
        header.submesh_count = u32::try_from(self.submeshes.len())?;
        header.draw_batch_count = u32::try_from(self.draw_batches.len())?;
        header.joint_palette_count = u32::try_from(self.joint_palette.len())?;
        header.index_buffer_size = u32::try_from(self.index_bytes.len())?;

        let mut out = EncodeBuffer::new();
        header.write(&mut out);
        for submesh in &self.submeshes {
            submesh.write(&mut out);
        }
        for batch in &self.draw_batches {
            batch.write(&mut out);
        }
        for joint in &self.joint_palette {
            joint.write(&mut out);
        }
        out.pad_to_align(8);
        out.write_bytes(&self.index_bytes);
        Ok(Bytes::from(out.into_inner()))
    }

    pub fn validate(
        &self,
        mesh_info: &MeshInfoHeader,
        material_slot_count: u32,
    ) -> AssetResult<()> {
        if self.header.index_count == 0 {
            return Err(AssetError::InvalidData(
                "empty render variant index buffer is invalid",
            ));
        }
        if self.header.primitive_topology != primitive_topology::TRIANGLE_LIST {
            return Err(AssetError::InvalidData("unsupported primitive topology"));
        }
        let index_size = match self.header.index_format {
            index_format::UINT16 => 2,
            index_format::UINT32 => 4,
            _ => return Err(AssetError::InvalidData("invalid index format")),
        };
        let expected_index_bytes = self
            .header
            .index_count
            .checked_mul(index_size)
            .ok_or(AssetError::OffsetOverflow)?;
        if self.index_bytes.len() != expected_index_bytes as usize {
            return Err(AssetError::InvalidData("invalid index buffer size"));
        }
        if !valid_bounds(self.header.bounds_min, self.header.bounds_max) {
            return Err(AssetError::InvalidData("invalid render variant bounds"));
        }
        validate_indices(
            &self.index_bytes,
            self.header.index_format,
            mesh_info.vertex_count,
        )?;

        for submesh in &self.submeshes {
            checked_range_u32(
                submesh.index_start,
                submesh.index_count,
                self.header.index_count,
            )?;
            checked_range_u32(
                submesh.joint_palette_start,
                submesh.joint_palette_count,
                self.joint_palette.len() as u32,
            )?;
            validate_material_slot(submesh.material_slot, material_slot_count)?;
        }
        for batch in &self.draw_batches {
            checked_range_u32(
                batch.index_start,
                batch.index_count,
                self.header.index_count,
            )?;
            if batch.submesh_index >= self.submeshes.len() as u32 {
                return Err(AssetError::InvalidData("draw batch submesh out of range"));
            }
            validate_material_slot(batch.material_slot, material_slot_count)?;
        }
        Ok(())
    }
}

fn read_f32x3(cursor: &mut DecodeCursor<'_>) -> AssetResult<[f32; 3]> {
    Ok([
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
        cursor.read_f32_le()?,
    ])
}

fn write_f32x3(out: &mut EncodeBuffer, value: [f32; 3]) {
    for item in value {
        out.write_f32_le(item);
    }
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

fn checked_range_u32(start: u32, count: u32, len: u32) -> AssetResult<()> {
    if start.checked_add(count).filter(|end| *end <= len).is_none() {
        return Err(AssetError::RangeOutOfBounds);
    }
    Ok(())
}

fn validate_material_slot(material_slot: u32, material_slot_count: u32) -> AssetResult<()> {
    if material_slot_count > 0 && material_slot >= material_slot_count {
        return Err(AssetError::InvalidData("material slot out of range"));
    }
    Ok(())
}

fn validate_indices(bytes: &Bytes, index_format: u32, vertex_count: u32) -> AssetResult<()> {
    match index_format {
        index_format::UINT16 => {
            for chunk in bytes.chunks_exact(2) {
                let index = u32::from(u16::from_le_bytes([chunk[0], chunk[1]]));
                if index >= vertex_count {
                    return Err(AssetError::InvalidData("index references missing vertex"));
                }
            }
            Ok(())
        }
        index_format::UINT32 => {
            for chunk in bytes.chunks_exact(4) {
                let index = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if index >= vertex_count {
                    return Err(AssetError::InvalidData("index references missing vertex"));
                }
            }
            Ok(())
        }
        _ => Err(AssetError::InvalidData("invalid index format")),
    }
}

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}
