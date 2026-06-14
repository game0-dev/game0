use bytes::Bytes;
use file_core::{
    align::{align_up, checked_range},
    AssetError, AssetRead, AssetResult, DecodeCursor, EncodeBuffer,
};

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
pub struct MeshInfoHeader {
    pub mesh_flags: u32,
    pub default_lod: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounding_sphere_center: [f32; 3],
    pub bounding_sphere_radius: f32,
    pub source_format: u32,
    pub source_version: u32,
    pub primitive_topology: u32,
    pub vertex_layout_id: u32,
    pub vertex_attribute_mask: u32,
    pub vertex_stride: u32,
    pub vertex_count: u32,
    pub vertex_buffer_size: u32,
}

impl MeshInfoHeader {
    pub const BYTE_SIZE: usize = 80;

    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        if bytes.len() < Self::BYTE_SIZE {
            return Err(file_core::AssetError::UnexpectedEof);
        }
        let mut cursor = DecodeCursor::new(&bytes[..Self::BYTE_SIZE]);
        Ok(Self {
            mesh_flags: cursor.read_u32_le()?,
            default_lod: cursor.read_u32_le()?,
            bounds_min: read_f32x3(&mut cursor)?,
            bounds_max: read_f32x3(&mut cursor)?,
            bounding_sphere_center: read_f32x3(&mut cursor)?,
            bounding_sphere_radius: cursor.read_f32_le()?,
            source_format: cursor.read_u32_le()?,
            source_version: cursor.read_u32_le()?,
            primitive_topology: cursor.read_u32_le()?,
            vertex_layout_id: cursor.read_u32_le()?,
            vertex_attribute_mask: cursor.read_u32_le()?,
            vertex_stride: cursor.read_u32_le()?,
            vertex_count: cursor.read_u32_le()?,
            vertex_buffer_size: cursor.read_u32_le()?,
        })
    }

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_u32_le(self.mesh_flags);
        out.write_u32_le(self.default_lod);
        write_f32x3(out, self.bounds_min);
        write_f32x3(out, self.bounds_max);
        write_f32x3(out, self.bounding_sphere_center);
        out.write_f32_le(self.bounding_sphere_radius);
        out.write_u32_le(self.source_format);
        out.write_u32_le(self.source_version);
        out.write_u32_le(self.primitive_topology);
        out.write_u32_le(self.vertex_layout_id);
        out.write_u32_le(self.vertex_attribute_mask);
        out.write_u32_le(self.vertex_stride);
        out.write_u32_le(self.vertex_count);
        out.write_u32_le(self.vertex_buffer_size);
    }

    pub fn vertex_buffer_offset(&self) -> AssetResult<u64> {
        Ok(u64::try_from(align_up(Self::BYTE_SIZE, 8)?)?)
    }
}

#[derive(Clone)]
pub struct MeshInfoReader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: R,
    offset: u64,
    len: u32,
    pub header: MeshInfoHeader,
}

impl<R> MeshInfoReader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn read(reader: R, offset: u64, len: u32) -> AssetResult<Self> {
        let header = MeshInfoHeader::read(
            reader
                .read_at(offset, MeshInfoHeader::BYTE_SIZE as u64)
                .await?,
        )?;
        Ok(Self {
            reader,
            offset,
            len,
            header,
        })
    }

    pub async fn vertex_bytes(&self) -> AssetResult<Bytes> {
        let offset = self.header.vertex_buffer_offset()?;
        let size = u64::from(self.header.vertex_buffer_size);
        checked_range(u64::from(self.len), offset, size)?;
        let absolute = self
            .offset
            .checked_add(offset)
            .ok_or(AssetError::OffsetOverflow)?;
        self.reader.read_at(absolute, size).await
    }

    pub async fn read_builder(&self) -> AssetResult<MeshInfoBuilder> {
        Ok(MeshInfoBuilder {
            header: self.header.clone(),
            vertex_bytes: self.vertex_bytes().await?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MeshInfoBuilder {
    pub header: MeshInfoHeader,
    pub vertex_bytes: Bytes,
}

impl MeshInfoBuilder {
    pub fn write(&self) -> AssetResult<Bytes> {
        let mut header = self.header.clone();
        header.vertex_buffer_size = u32::try_from(self.vertex_bytes.len())?;

        let mut out = EncodeBuffer::new();
        header.write(&mut out);
        out.pad_to_align(8);
        out.write_bytes(&self.vertex_bytes);
        Ok(Bytes::from(out.into_inner()))
    }

    pub fn validate(&self) -> AssetResult<()> {
        if self.header.vertex_stride == 0 {
            return Err(AssetError::InvalidData(
                "vertex_stride must be greater than zero",
            ));
        }
        if self.header.vertex_count == 0 {
            return Err(AssetError::InvalidData(
                "empty mesh vertex buffer is invalid",
            ));
        }
        if self.header.primitive_topology != primitive_topology::TRIANGLE_LIST {
            return Err(AssetError::InvalidData("unsupported primitive topology"));
        }
        let expected_len = self
            .header
            .vertex_count
            .checked_mul(self.header.vertex_stride)
            .ok_or(AssetError::OffsetOverflow)?;
        if self.vertex_bytes.len() != expected_len as usize {
            return Err(AssetError::InvalidData("invalid vertex buffer size"));
        }
        if !valid_bounds(self.header.bounds_min, self.header.bounds_max) {
            return Err(AssetError::InvalidData("invalid mesh bounds"));
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

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}
