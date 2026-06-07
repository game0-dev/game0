use file_core::{align::align_up, AssetResult, EncodeBuffer};
use bytes::Bytes;

use crate::sections::*;

pub fn encode_info_section(section: &Mesh0InfoSectionOwned) -> AssetResult<Bytes> {
    let info = &section.info;
    let mut out = EncodeBuffer::new();
    out.write_u32_le(info.mesh_flags);
    out.write_u32_le(info.lod_count);
    out.write_u32_le(info.default_lod);
    out.write_u32_le(info.material_slot_count);
    out.write_u32_le(info.skinning_section_count);
    out.write_u32_le(info.source_feature_count);
    write_f32x3(&mut out, info.bounds_min);
    write_f32x3(&mut out, info.bounds_max);
    write_f32x3(&mut out, info.bounding_sphere_center);
    out.write_f32_le(info.bounding_sphere_radius);
    out.write_u32_le(info.source_format);
    out.write_u32_le(info.source_version);
    Ok(Bytes::from(out.into_inner()))
}

pub fn encode_material_slots_section(
    section: &Mesh0MaterialSlotsSectionOwned,
) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for slot in &section.slots {
        out.write_u32_le(slot.slot_index);
        out.write_u32_le(slot.flags);
        out.write_asset_id128(slot.material_asset);
        out.write_u32_le(slot.render_queue);
        out.write_u32_le(slot.shader_hint);
        out.write_u32_le(slot.source_material_index);
        out.write_u32_le(slot.source_texture_combo_index);
        out.write_u32_le(slot.source_texture_count);
        out.write_u64_le(slot.name_hash);
    }
    Ok(Bytes::from(out.into_inner()))
}

pub fn encode_skinning_section(section: &Mesh0SkinningSectionOwned) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    out.write_asset_id128(section.skeleton_asset);
    out.write_u32_le(section.flags);
    out.write_u32_le(section.joint_count_hint);
    out.write_u32_le(section.max_weights_per_vertex);
    out.write_u32_le(section.joint_index_format);
    out.write_u32_le(section.weight_format);
    out.write_u32_le(section.source_bone_count);
    out.write_u32_le(section.source_key_bone_count);
    Ok(Bytes::from(out.into_inner()))
}

pub fn encode_raw_asset_refs(refs: &[Mesh0AssetRef]) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for item in refs {
        out.write_asset_id128(item.asset);
        out.write_u32_le(item.flags);
        out.write_u32_le(item.kind);
        out.write_u32_le(item.source_index);
    }
    Ok(Bytes::from(out.into_inner()))
}

pub fn encode_source_features_section(
    section: &Mesh0SourceFeaturesSectionOwned,
) -> AssetResult<Bytes> {
    let mut out = EncodeBuffer::new();
    for item in &section.features {
        out.write_u32_le(item.feature_kind);
        out.write_u32_le(item.support_status);
        out.write_u32_le(item.mapped_target_kind);
        out.write_u32_le(item.mapped_target_index);
        out.write_u32_le(item.source_index);
        out.write_u32_le(item.flags);
    }
    Ok(Bytes::from(out.into_inner()))
}

pub fn encode_lod_section(section: &Mesh0LodSectionOwned) -> AssetResult<Bytes> {
    let mut body = vec![0; Mesh0LodHeader::BYTE_SIZE];
    let submeshes = append_table(&mut body, &section.submeshes, encode_submesh)?;
    let draw_batches = append_table(&mut body, &section.draw_batches, encode_draw_batch)?;
    let joint_palette = append_table(&mut body, &section.joint_palette, encode_joint_palette)?;
    let vertex_buffer = append_bytes(&mut body, &section.vertex_bytes)?;
    let index_buffer = append_bytes(&mut body, &section.index_bytes)?;

    let mut header = section.header.clone();
    header.submeshes = submeshes;
    header.draw_batches = draw_batches;
    header.joint_palette = joint_palette;
    header.vertex_buffer = vertex_buffer;
    header.index_buffer = index_buffer;

    let mut header_bytes = EncodeBuffer::new();
    encode_lod_header(&header, &mut header_bytes);
    body[..Mesh0LodHeader::BYTE_SIZE].copy_from_slice(&header_bytes.into_inner());
    Ok(Bytes::from(body))
}

fn append_table<T>(
    body: &mut Vec<u8>,
    values: &[T],
    encode: fn(&T, &mut EncodeBuffer),
) -> AssetResult<TableSpan> {
    let offset = align_up(body.len(), 8)?;
    body.resize(offset, 0);
    let mut table = EncodeBuffer::new();
    for value in values {
        encode(value, &mut table);
    }
    let bytes = table.into_inner();
    let stride = if values.is_empty() {
        0
    } else {
        bytes.len() / values.len()
    };
    body.extend_from_slice(&bytes);
    Ok(TableSpan {
        offset: u32::try_from(offset)?,
        count: u32::try_from(values.len())?,
        stride: u32::try_from(stride)?,
    })
}

fn append_bytes(body: &mut Vec<u8>, bytes: &Bytes) -> AssetResult<LocalBlobSpan> {
    let offset = align_up(body.len(), 8)?;
    body.resize(offset, 0);
    body.extend_from_slice(bytes);
    Ok(LocalBlobSpan {
        offset: u32::try_from(offset)?,
        size: u32::try_from(bytes.len())?,
    })
}

fn encode_lod_header(header: &Mesh0LodHeader, out: &mut EncodeBuffer) {
    out.write_u32_le(header.lod_level);
    out.write_u32_le(header.lod_flags);
    out.write_f32_le(header.screen_size);
    out.write_f32_le(header.max_distance);
    out.write_u32_le(header.primitive_topology);
    out.write_u32_le(header.vertex_layout_id);
    out.write_u32_le(header.vertex_attribute_mask);
    out.write_u32_le(header.vertex_stride);
    out.write_u32_le(header.vertex_count);
    out.write_u32_le(header.index_count);
    out.write_u32_le(header.index_format);
    write_f32x3(out, header.bounds_min);
    write_f32x3(out, header.bounds_max);
    write_f32x3(out, header.bounding_sphere_center);
    out.write_f32_le(header.bounding_sphere_radius);
    write_table_span(out, header.submeshes);
    write_table_span(out, header.draw_batches);
    write_table_span(out, header.joint_palette);
    write_blob_span(out, header.vertex_buffer);
    write_blob_span(out, header.index_buffer);
}

fn encode_submesh(value: &Mesh0Submesh, out: &mut EncodeBuffer) {
    out.write_u32_le(value.submesh_id);
    out.write_u32_le(value.flags);
    out.write_u32_le(value.vertex_start);
    out.write_u32_le(value.vertex_count);
    out.write_u32_le(value.index_start);
    out.write_u32_le(value.index_count);
    out.write_u32_le(value.material_slot);
    out.write_u32_le(value.joint_palette_start);
    out.write_u32_le(value.joint_palette_count);
    out.write_u32_le(value.max_bone_influence);
    write_f32x3(out, value.center);
    write_f32x3(out, value.sort_center);
    out.write_f32_le(value.bounding_radius);
    out.write_u32_le(value.source_submesh_id);
    out.write_u32_le(value.source_level);
}

fn encode_draw_batch(value: &Mesh0DrawBatch, out: &mut EncodeBuffer) {
    out.write_u32_le(value.batch_id);
    out.write_u32_le(value.flags);
    out.write_u32_le(value.submesh_index);
    out.write_u32_le(value.material_slot);
    out.write_u32_le(value.render_queue);
    out.write_u32_le(value.shader_hint);
    out.write_i32_le(value.priority);
    out.write_u32_le(value.vertex_start);
    out.write_u32_le(value.vertex_count);
    out.write_u32_le(value.index_start);
    out.write_u32_le(value.index_count);
    out.write_u32_le(value.source_skin_batch_index);
    out.write_u32_le(value.source_skin_section_index);
    out.write_u32_le(value.source_geoset_index);
    out.write_u32_le(value.source_material_index);
    out.write_u32_le(value.source_texture_combo_index);
}

fn encode_joint_palette(value: &Mesh0JointPaletteEntry, out: &mut EncodeBuffer) {
    out.write_u32_le(value.local_joint_index);
    out.write_u32_le(value.skeleton_joint_index);
    out.write_u32_le(value.source_bone_index);
    out.write_u32_le(value.flags);
}

fn write_table_span(out: &mut EncodeBuffer, span: TableSpan) {
    out.write_u32_le(span.offset);
    out.write_u32_le(span.count);
    out.write_u32_le(span.stride);
}

fn write_blob_span(out: &mut EncodeBuffer, span: LocalBlobSpan) {
    out.write_u32_le(span.offset);
    out.write_u32_le(span.size);
}

fn write_f32x3(out: &mut EncodeBuffer, value: [f32; 3]) {
    for item in value {
        out.write_f32_le(item);
    }
}
