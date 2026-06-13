use bytes::Bytes;
use file_core::{read_f32x3, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
pub struct MeshInfoSection {
    pub mesh_flags: u32,
    pub default_lod: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounding_sphere_center: [f32; 3],
    pub bounding_sphere_radius: f32,
    pub source_format: u32,
    pub source_version: u32,
}

impl MeshInfoSection {
    pub const BYTE_SIZE: usize = 56;

    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        let mut cursor = DecodeCursor::new(&bytes);
        Ok(Self {
            mesh_flags: cursor.read_u32_le()?,
            default_lod: cursor.read_u32_le()?,
            bounds_min: read_f32x3(&mut cursor)?,
            bounds_max: read_f32x3(&mut cursor)?,
            bounding_sphere_center: read_f32x3(&mut cursor)?,
            bounding_sphere_radius: cursor.read_f32_le()?,
            source_format: cursor.read_u32_le()?,
            source_version: cursor.read_u32_le()?,
        })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        out.write_u32_le(self.mesh_flags);
        out.write_u32_le(self.default_lod);
        out.write_f32x3(self.bounds_min);
        out.write_f32x3(self.bounds_max);
        out.write_f32x3(self.bounding_sphere_center);
        out.write_f32_le(self.bounding_sphere_radius);
        out.write_u32_le(self.source_format);
        out.write_u32_le(self.source_version);
        Ok(Bytes::from(out.into_inner()))
    }
}
