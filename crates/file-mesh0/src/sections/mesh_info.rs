use file_core::{AssetReader, AssetResult, DecodeCursor, EncodeBuffer};

#[derive(Debug, Clone)]
pub struct MeshInfoHeader {
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub bounding_sphere_radius: f32,
}

impl MeshInfoHeader {
    pub const BYTE_SIZE: u64 = 28;

    pub async fn read<R>(reader: R) -> AssetResult<Self>
    where
        R: AssetReader,
    {
        let mut cursor = DecodeCursor::from_reader(&reader, Self::BYTE_SIZE).await?;
        Ok(Self {
            bounding_box_min: cursor.read_f32x3()?,
            bounding_box_max: cursor.read_f32x3()?,
            bounding_sphere_radius: cursor.read_f32_le()?,
        })
    }

    pub fn write(&self, out: &mut EncodeBuffer) {
        out.write_f32x3(self.bounding_box_min);
        out.write_f32x3(self.bounding_box_max);
        out.write_f32_le(self.bounding_sphere_radius);
    }
}
