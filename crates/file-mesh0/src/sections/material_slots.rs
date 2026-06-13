use bytes::Bytes;
use file_core::{decode_table, AssetResult, DecodeCursor, EncodeBuffer};

pub mod render_queue {
    pub const OPAQUE: u32 = 1;
    pub const ALPHA_TEST: u32 = 2;
    pub const TRANSPARENT: u32 = 3;
    pub const ADDITIVE: u32 = 4;
}

#[derive(Debug, Clone)]
pub struct Mesh0MaterialSlot {
    pub slot_index: u32,
    pub flags: u32,
    pub material_asset: u64,
    pub render_queue: u32,
    pub shader_hint: u32,
    pub source_material_index: u32,
    pub source_texture_combo_index: u32,
    pub source_texture_count: u32,
    pub name_hash: u64,
}

impl Mesh0MaterialSlot {
    pub const BYTE_SIZE: usize = 44;
}

#[derive(Debug, Clone)]
pub struct MaterialSlotsSection {
    pub slots: Vec<Mesh0MaterialSlot>,
}

impl MaterialSlotsSection {
    pub fn read(bytes: Bytes) -> AssetResult<Self> {
        Ok(Self {
            slots: decode_table(&bytes, Mesh0MaterialSlot::BYTE_SIZE, decode_material_slot)?,
        })
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        let mut out = EncodeBuffer::new();
        for slot in &self.slots {
            out.write_u32_le(slot.slot_index);
            out.write_u32_le(slot.flags);
            out.write_u64_le(slot.material_asset);
            out.write_u32_le(slot.render_queue);
            out.write_u32_le(slot.shader_hint);
            out.write_u32_le(slot.source_material_index);
            out.write_u32_le(slot.source_texture_combo_index);
            out.write_u32_le(slot.source_texture_count);
            out.write_u64_le(slot.name_hash);
        }
        Ok(Bytes::from(out.into_inner()))
    }
}

fn decode_material_slot(cursor: &mut DecodeCursor<'_>) -> AssetResult<Mesh0MaterialSlot> {
    Ok(Mesh0MaterialSlot {
        slot_index: cursor.read_u32_le()?,
        flags: cursor.read_u32_le()?,
        material_asset: cursor.read_u64_le()?,
        render_queue: cursor.read_u32_le()?,
        shader_hint: cursor.read_u32_le()?,
        source_material_index: cursor.read_u32_le()?,
        source_texture_combo_index: cursor.read_u32_le()?,
        source_texture_count: cursor.read_u32_le()?,
        name_hash: cursor.read_u64_le()?,
    })
}
