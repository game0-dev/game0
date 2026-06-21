use bytes::Bytes;
use file_core::{AssetError, AssetReader, AssetResult, DecodeCursor, EncodeBuffer};

use super::mesh_info::MeshInfoHeader;

pub mod render_queue {
    pub const OPAQUE: u32 = 1;
    pub const ALPHA_TEST: u32 = 2;
    pub const TRANSPARENT: u32 = 3;
    pub const ADDITIVE: u32 = 4;
}

#[derive(Debug, Clone)]
struct RenderVariantHeader {
    submesh_count: u32,
    draw_batch_count: u32,
    joint_palette_count: u32,
    vertex_buffer_size: u32,
    index_buffer_size: u32,

    submesh_offset: u64,
    submesh_len: u64,
    draw_batch_offset: u64,
    draw_batch_len: u64,
    joint_palette_offset: u64,
    joint_palette_len: u64,
    vertex_buffer_offset: u64,
    vertex_buffer_len: u64,
    index_buffer_offset: u64,
    index_buffer_len: u64,
}

impl RenderVariantHeader {
    const BYTE_SIZE: u64 = 20;

    async fn read<R>(reader: &R) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let mut cursor = DecodeCursor::from_reader(reader, Self::BYTE_SIZE).await?;
        Self::new(
            cursor.read_u32_le()?,
            cursor.read_u32_le()?,
            cursor.read_u32_le()?,
            cursor.read_u32_le()?,
            cursor.read_u32_le()?,
        )
    }

    fn from_builder(builder: &RenderVariantBuilder) -> AssetResult<Self> {
        Self::new(
            u32::try_from(builder.submeshes.len())?,
            u32::try_from(builder.draw_batches.len())?,
            u32::try_from(builder.joint_palette.len())?,
            u32::try_from(builder.vertex_bytes.len())?,
            u32::try_from(builder.index_bytes.len())?,
        )
    }

    fn new(
        submesh_count: u32,
        draw_batch_count: u32,
        joint_palette_count: u32,
        vertex_buffer_size: u32,
        index_buffer_size: u32,
    ) -> AssetResult<Self> {
        let mut header = Self {
            submesh_count,
            draw_batch_count,
            joint_palette_count,
            vertex_buffer_size,
            index_buffer_size,
            submesh_offset: 0,
            submesh_len: 0,
            draw_batch_offset: 0,
            draw_batch_len: 0,
            joint_palette_offset: 0,
            joint_palette_len: 0,
            vertex_buffer_offset: 0,
            vertex_buffer_len: 0,
            index_buffer_offset: 0,
            index_buffer_len: 0,
        };
        header.calculate_offsets()?;
        Ok(header)
    }

    fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.submesh_count);
        out.write_u32_le(self.draw_batch_count);
        out.write_u32_le(self.joint_palette_count);
        out.write_u32_le(self.vertex_buffer_size);
        out.write_u32_le(self.index_buffer_size);
    }

    fn calculate_offsets(&mut self) -> AssetResult<()> {
        self.submesh_offset = Self::BYTE_SIZE;
        self.submesh_len = u64::from(self.submesh_count)
            .checked_mul(Mesh0Submesh::BYTE_SIZE)
            .ok_or(AssetError::OffsetOverflow)?;

        self.draw_batch_offset = self
            .submesh_offset
            .checked_add(self.submesh_len)
            .ok_or(AssetError::OffsetOverflow)?;
        self.draw_batch_len = u64::from(self.draw_batch_count)
            .checked_mul(Mesh0DrawBatch::BYTE_SIZE)
            .ok_or(AssetError::OffsetOverflow)?;

        self.joint_palette_offset = self
            .draw_batch_offset
            .checked_add(self.draw_batch_len)
            .ok_or(AssetError::OffsetOverflow)?;
        self.joint_palette_len = u64::from(self.joint_palette_count)
            .checked_mul(Mesh0JointPaletteEntry::BYTE_SIZE)
            .ok_or(AssetError::OffsetOverflow)?;

        let metadata_end = self
            .joint_palette_offset
            .checked_add(self.joint_palette_len)
            .ok_or(AssetError::OffsetOverflow)?;
        let padded_metadata_end = metadata_end
            .checked_add(7)
            .ok_or(AssetError::OffsetOverflow)?;
        self.vertex_buffer_offset = padded_metadata_end / 8 * 8;
        self.vertex_buffer_len = u64::from(self.vertex_buffer_size);

        let vertex_buffer_end = self
            .vertex_buffer_offset
            .checked_add(self.vertex_buffer_len)
            .ok_or(AssetError::OffsetOverflow)?;
        let padded_vertex_buffer_end = vertex_buffer_end
            .checked_add(7)
            .ok_or(AssetError::OffsetOverflow)?;
        self.index_buffer_offset = padded_vertex_buffer_end / 8 * 8;
        self.index_buffer_len = u64::from(self.index_buffer_size);

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
}

impl Mesh0Submesh {
    pub const BYTE_SIZE: u64 = 60;

    pub fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
        Ok(Self {
            submesh_id: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
            index_start: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            material_slot: cursor.read_u32_le()?,
            joint_palette_start: cursor.read_u32_le()?,
            joint_palette_count: cursor.read_u32_le()?,
            max_bone_influence: cursor.read_u32_le()?,
            center: cursor.read_f32x3()?,
            sort_center: cursor.read_f32x3()?,
            bounding_radius: cursor.read_f32_le()?,
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
        out.write_f32x3(self.center);
        out.write_f32x3(self.sort_center);
        out.write_f32_le(self.bounding_radius);
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
    pub skin_batch_index: u32,
    pub skin_section_index: u32,
    pub geoset_index: u32,
    pub texture_combo_index: u32,
}

impl Mesh0DrawBatch {
    pub const BYTE_SIZE: u64 = 52;

    pub fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
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
            skin_batch_index: cursor.read_u32_le()?,
            skin_section_index: cursor.read_u32_le()?,
            geoset_index: cursor.read_u32_le()?,
            texture_combo_index: cursor.read_u32_le()?,
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
        out.write_u32_le(self.skin_batch_index);
        out.write_u32_le(self.skin_section_index);
        out.write_u32_le(self.geoset_index);
        out.write_u32_le(self.texture_combo_index);
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
    pub const BYTE_SIZE: u64 = 16;

    pub fn read(cursor: &mut DecodeCursor) -> AssetResult<Self> {
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
    R: AssetReader,
{
    reader: R,
    vertex_buffer_offset: u64,
    index_buffer_offset: u64,
    vertex_buffer_size: u64,
    index_buffer_size: u64,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
}

impl<R> RenderVariantReader<R>
where
    R: AssetReader,
{
    pub const VERTEX_SIZE: usize = 48;
    pub const INDEX_SIZE: usize = 2;

    pub async fn read(reader: R) -> AssetResult<Self> {
        let header = RenderVariantHeader::read(&reader).await?;

        let metadata_end = header
            .joint_palette_offset
            .checked_add(header.joint_palette_len)
            .ok_or(AssetError::OffsetOverflow)?;
        let metadata = reader.read_at(0, metadata_end).await?;
        let metadata_start = usize::try_from(header.submesh_offset)?;
        let metadata_end = usize::try_from(metadata_end)?;
        let mut cursor = DecodeCursor::new(metadata.slice(metadata_start..metadata_end));

        let mut submeshes = Vec::with_capacity(usize::try_from(header.submesh_count)?);
        for _ in 0..header.submesh_count {
            submeshes.push(Mesh0Submesh::read(&mut cursor)?);
        }
        let mut draw_batches = Vec::with_capacity(usize::try_from(header.draw_batch_count)?);
        for _ in 0..header.draw_batch_count {
            draw_batches.push(Mesh0DrawBatch::read(&mut cursor)?);
        }
        let mut joint_palette = Vec::with_capacity(usize::try_from(header.joint_palette_count)?);
        for _ in 0..header.joint_palette_count {
            joint_palette.push(Mesh0JointPaletteEntry::read(&mut cursor)?);
        }
        Ok(Self {
            reader,
            vertex_buffer_offset: header.vertex_buffer_offset,
            index_buffer_offset: header.index_buffer_offset,
            vertex_buffer_size: header.vertex_buffer_len,
            index_buffer_size: header.index_buffer_len,
            submeshes,
            draw_batches,
            joint_palette,
        })
    }

    pub async fn index_bytes(&self) -> AssetResult<Bytes> {
        self.reader
            .read_at(self.index_buffer_offset, self.index_buffer_size)
            .await
    }

    pub async fn vertex_bytes(&self) -> AssetResult<Bytes> {
        self.reader
            .read_at(self.vertex_buffer_offset, self.vertex_buffer_size)
            .await
    }

    pub async fn read_builder(&self) -> AssetResult<RenderVariantBuilder> {
        Ok(RenderVariantBuilder {
            submeshes: self.submeshes.clone(),
            draw_batches: self.draw_batches.clone(),
            joint_palette: self.joint_palette.clone(),
            vertex_bytes: self.vertex_bytes().await?,
            index_bytes: self.index_bytes().await?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RenderVariantBuilder {
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
    pub vertex_bytes: Bytes,
    pub index_bytes: Bytes,
}

impl RenderVariantBuilder {
    pub fn write(&self) -> AssetResult<Bytes> {
        let header = RenderVariantHeader::from_builder(self)?;

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
        out.write_bytes(&self.vertex_bytes);
        out.pad_to_align(8);
        out.write_bytes(&self.index_bytes);
        Ok(Bytes::from(out.into_inner()))
    }

    pub fn validate(&self, _mesh_info: &MeshInfoHeader) -> AssetResult<()> {
        let vertex_count = u32::try_from(self.vertex_bytes.len() / 48)?;
        let index_count = u32::try_from(self.index_bytes.len() / 2)?;
        if index_count == 0 {
            return Err(AssetError::InvalidData(
                "empty render variant index buffer is invalid",
            ));
        }
        if vertex_count == 0 {
            return Err(AssetError::InvalidData(
                "empty render variant vertex buffer is invalid",
            ));
        }
        if self.vertex_bytes.len() % 48 != 0 {
            return Err(AssetError::InvalidData("invalid vertex buffer size"));
        }
        if self.index_bytes.len() % 2 != 0 {
            return Err(AssetError::InvalidData("invalid index buffer size"));
        }
        self.validate_indices(vertex_count)?;

        for submesh in &self.submeshes {
            Self::checked_range_u32(submesh.index_start, submesh.index_count, index_count)?;
            Self::checked_range_u32(
                submesh.joint_palette_start,
                submesh.joint_palette_count,
                u32::try_from(self.joint_palette.len())?,
            )?;
        }
        for batch in &self.draw_batches {
            Self::checked_range_u32(batch.index_start, batch.index_count, index_count)?;
            if batch.submesh_index >= u32::try_from(self.submeshes.len())? {
                return Err(AssetError::InvalidData("draw batch submesh out of range"));
            }
        }
        Ok(())
    }

    fn checked_range_u32(start: u32, count: u32, len: u32) -> AssetResult<()> {
        if start.checked_add(count).filter(|end| *end <= len).is_none() {
            return Err(AssetError::RangeOutOfBounds);
        }
        Ok(())
    }

    fn validate_indices(&self, vertex_count: u32) -> AssetResult<()> {
        for chunk in self.index_bytes.chunks_exact(2) {
            let index = u32::from(u16::from_le_bytes([chunk[0], chunk[1]]));
            if index >= vertex_count {
                return Err(AssetError::InvalidData("index references missing vertex"));
            }
        }
        Ok(())
    }
}
