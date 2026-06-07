use file_core::{align::checked_range, AssetError, AssetResult, DecodeCursor};
use bytes::Bytes;

use crate::sections::*;

pub fn decode_info_section(bytes: Bytes) -> AssetResult<Mesh0InfoSectionOwned> {
    let mut c = DecodeCursor::new(&bytes);
    let info = Mesh0Info {
        mesh_flags: c.read_u32_le()?,
        lod_count: c.read_u32_le()?,
        default_lod: c.read_u32_le()?,
        material_slot_count: c.read_u32_le()?,
        skinning_section_count: c.read_u32_le()?,
        source_feature_count: c.read_u32_le()?,
        bounds_min: read_f32x3(&mut c)?,
        bounds_max: read_f32x3(&mut c)?,
        bounding_sphere_center: read_f32x3(&mut c)?,
        bounding_sphere_radius: c.read_f32_le()?,
        source_format: c.read_u32_le()?,
        source_version: c.read_u32_le()?,
    };
    Ok(Mesh0InfoSectionOwned { info })
}

pub fn decode_material_slots_section(bytes: Bytes) -> AssetResult<Mesh0MaterialSlotsSectionOwned> {
    Ok(Mesh0MaterialSlotsSectionOwned {
        slots: decode_table(&bytes, Mesh0MaterialSlot::BYTE_SIZE, decode_material_slot)?,
    })
}

pub fn decode_skinning_section(bytes: Bytes) -> AssetResult<Mesh0SkinningSectionOwned> {
    let mut c = DecodeCursor::new(&bytes);
    Ok(Mesh0SkinningSectionOwned {
        skeleton_asset: c.read_asset_id128()?,
        flags: c.read_u32_le()?,
        joint_count_hint: c.read_u32_le()?,
        max_weights_per_vertex: c.read_u32_le()?,
        joint_index_format: c.read_u32_le()?,
        weight_format: c.read_u32_le()?,
        source_bone_count: c.read_u32_le()?,
        source_key_bone_count: c.read_u32_le()?,
    })
}

pub fn decode_asset_refs(bytes: Bytes) -> AssetResult<Vec<Mesh0AssetRef>> {
    decode_table(&bytes, Mesh0AssetRef::BYTE_SIZE, decode_asset_ref)
}

pub fn decode_source_features_section(
    bytes: Bytes,
) -> AssetResult<Mesh0SourceFeaturesSectionOwned> {
    Ok(Mesh0SourceFeaturesSectionOwned {
        features: decode_table(&bytes, Mesh0SourceFeature::BYTE_SIZE, decode_source_feature)?,
    })
}

pub fn decode_lod_header(bytes: Bytes) -> AssetResult<Mesh0LodHeader> {
    if bytes.len() != Mesh0LodHeader::BYTE_SIZE {
        return Err(AssetError::UnexpectedEof);
    }
    let mut c = DecodeCursor::new(&bytes);
    Ok(Mesh0LodHeader {
        lod_level: c.read_u32_le()?,
        lod_flags: c.read_u32_le()?,
        screen_size: c.read_f32_le()?,
        max_distance: c.read_f32_le()?,
        primitive_topology: c.read_u32_le()?,
        vertex_layout_id: c.read_u32_le()?,
        vertex_attribute_mask: c.read_u32_le()?,
        vertex_stride: c.read_u32_le()?,
        vertex_count: c.read_u32_le()?,
        index_count: c.read_u32_le()?,
        index_format: c.read_u32_le()?,
        bounds_min: read_f32x3(&mut c)?,
        bounds_max: read_f32x3(&mut c)?,
        bounding_sphere_center: read_f32x3(&mut c)?,
        bounding_sphere_radius: c.read_f32_le()?,
        submeshes: read_table_span(&mut c)?,
        draw_batches: read_table_span(&mut c)?,
        joint_palette: read_table_span(&mut c)?,
        vertex_buffer: read_blob_span(&mut c)?,
        index_buffer: read_blob_span(&mut c)?,
    })
}

pub fn decode_lod_section(bytes: Bytes) -> AssetResult<Mesh0LodSectionOwned> {
    let header = decode_lod_header(bytes.slice(0..Mesh0LodHeader::BYTE_SIZE))?;
    let submeshes = read_table_span_owned(
        &bytes,
        header.submeshes,
        Mesh0Submesh::BYTE_SIZE,
        decode_submesh,
    )?;
    let draw_batches = read_table_span_owned(
        &bytes,
        header.draw_batches,
        Mesh0DrawBatch::BYTE_SIZE,
        decode_draw_batch,
    )?;
    let joint_palette = read_table_span_owned(
        &bytes,
        header.joint_palette,
        Mesh0JointPaletteEntry::BYTE_SIZE,
        decode_joint_palette_entry,
    )?;
    let vertex_range = checked_range(
        bytes.len() as u64,
        u64::from(header.vertex_buffer.offset),
        u64::from(header.vertex_buffer.size),
    )?;
    let index_range = checked_range(
        bytes.len() as u64,
        u64::from(header.index_buffer.offset),
        u64::from(header.index_buffer.size),
    )?;
    Ok(Mesh0LodSectionOwned {
        header,
        submeshes,
        draw_batches,
        joint_palette,
        vertex_bytes: bytes.slice(vertex_range),
        index_bytes: bytes.slice(index_range),
    })
}

fn read_table_span_owned<T>(
    bytes: &Bytes,
    span: TableSpan,
    expected_stride: usize,
    decode: fn(&mut DecodeCursor<'_>) -> AssetResult<T>,
) -> AssetResult<Vec<T>> {
    if span.stride != expected_stride as u32 {
        return Err(AssetError::InvalidData("invalid table stride"));
    }
    let size = u64::from(span.count)
        .checked_mul(u64::from(span.stride))
        .ok_or(AssetError::OffsetOverflow)?;
    let range = checked_range(bytes.len() as u64, u64::from(span.offset), size)?;
    decode_table(&bytes[range], expected_stride, decode)
}

pub fn decode_table<T>(
    bytes: &[u8],
    stride: usize,
    decode: fn(&mut DecodeCursor<'_>) -> AssetResult<T>,
) -> AssetResult<Vec<T>> {
    if stride == 0 || bytes.len() % stride != 0 {
        return Err(AssetError::InvalidData("invalid table size"));
    }
    let mut c = DecodeCursor::new(bytes);
    let mut values = Vec::with_capacity(bytes.len() / stride);
    while c.remaining() > 0 {
        values.push(decode(&mut c)?);
    }
    Ok(values)
}

fn decode_material_slot(c: &mut DecodeCursor<'_>) -> AssetResult<Mesh0MaterialSlot> {
    Ok(Mesh0MaterialSlot {
        slot_index: c.read_u32_le()?,
        flags: c.read_u32_le()?,
        material_asset: c.read_asset_id128()?,
        render_queue: c.read_u32_le()?,
        shader_hint: c.read_u32_le()?,
        source_material_index: c.read_u32_le()?,
        source_texture_combo_index: c.read_u32_le()?,
        source_texture_count: c.read_u32_le()?,
        name_hash: c.read_u64_le()?,
    })
}

fn decode_asset_ref(c: &mut DecodeCursor<'_>) -> AssetResult<Mesh0AssetRef> {
    Ok(Mesh0AssetRef {
        asset: c.read_asset_id128()?,
        flags: c.read_u32_le()?,
        kind: c.read_u32_le()?,
        source_index: c.read_u32_le()?,
    })
}

fn decode_source_feature(c: &mut DecodeCursor<'_>) -> AssetResult<Mesh0SourceFeature> {
    Ok(Mesh0SourceFeature {
        feature_kind: c.read_u32_le()?,
        support_status: c.read_u32_le()?,
        mapped_target_kind: c.read_u32_le()?,
        mapped_target_index: c.read_u32_le()?,
        source_index: c.read_u32_le()?,
        flags: c.read_u32_le()?,
    })
}

fn decode_submesh(c: &mut DecodeCursor<'_>) -> AssetResult<Mesh0Submesh> {
    Ok(Mesh0Submesh {
        submesh_id: c.read_u32_le()?,
        flags: c.read_u32_le()?,
        vertex_start: c.read_u32_le()?,
        vertex_count: c.read_u32_le()?,
        index_start: c.read_u32_le()?,
        index_count: c.read_u32_le()?,
        material_slot: c.read_u32_le()?,
        joint_palette_start: c.read_u32_le()?,
        joint_palette_count: c.read_u32_le()?,
        max_bone_influence: c.read_u32_le()?,
        center: read_f32x3(c)?,
        sort_center: read_f32x3(c)?,
        bounding_radius: c.read_f32_le()?,
        source_submesh_id: c.read_u32_le()?,
        source_level: c.read_u32_le()?,
    })
}

fn decode_draw_batch(c: &mut DecodeCursor<'_>) -> AssetResult<Mesh0DrawBatch> {
    Ok(Mesh0DrawBatch {
        batch_id: c.read_u32_le()?,
        flags: c.read_u32_le()?,
        submesh_index: c.read_u32_le()?,
        material_slot: c.read_u32_le()?,
        render_queue: c.read_u32_le()?,
        shader_hint: c.read_u32_le()?,
        priority: c.read_i32_le()?,
        vertex_start: c.read_u32_le()?,
        vertex_count: c.read_u32_le()?,
        index_start: c.read_u32_le()?,
        index_count: c.read_u32_le()?,
        source_skin_batch_index: c.read_u32_le()?,
        source_skin_section_index: c.read_u32_le()?,
        source_geoset_index: c.read_u32_le()?,
        source_material_index: c.read_u32_le()?,
        source_texture_combo_index: c.read_u32_le()?,
    })
}

fn decode_joint_palette_entry(c: &mut DecodeCursor<'_>) -> AssetResult<Mesh0JointPaletteEntry> {
    Ok(Mesh0JointPaletteEntry {
        local_joint_index: c.read_u32_le()?,
        skeleton_joint_index: c.read_u32_le()?,
        source_bone_index: c.read_u32_le()?,
        flags: c.read_u32_le()?,
    })
}

fn read_table_span(c: &mut DecodeCursor<'_>) -> AssetResult<TableSpan> {
    Ok(TableSpan {
        offset: c.read_u32_le()?,
        count: c.read_u32_le()?,
        stride: c.read_u32_le()?,
    })
}

fn read_blob_span(c: &mut DecodeCursor<'_>) -> AssetResult<LocalBlobSpan> {
    Ok(LocalBlobSpan {
        offset: c.read_u32_le()?,
        size: c.read_u32_le()?,
    })
}

fn read_f32x3(c: &mut DecodeCursor<'_>) -> AssetResult<[f32; 3]> {
    Ok([c.read_f32_le()?, c.read_f32_le()?, c.read_f32_le()?])
}

impl Mesh0Info {
    pub const BYTE_SIZE: usize = 72;
}
impl Mesh0MaterialSlot {
    pub const BYTE_SIZE: usize = 52;
}
impl Mesh0SkinningSectionOwned {
    pub const BYTE_SIZE: usize = 44;
}
impl Mesh0AssetRef {
    pub const BYTE_SIZE: usize = 28;
}
impl Mesh0SourceFeature {
    pub const BYTE_SIZE: usize = 24;
}
impl Mesh0LodHeader {
    pub const BYTE_SIZE: usize = 136;
}
impl Mesh0Submesh {
    pub const BYTE_SIZE: usize = 76;
}
impl Mesh0DrawBatch {
    pub const BYTE_SIZE: usize = 64;
}
impl Mesh0JointPaletteEntry {
    pub const BYTE_SIZE: usize = 16;
}
