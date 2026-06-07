use std::collections::HashSet;

use file_core::{AssetError, AssetResult};

use crate::{
    format::{index_format, primitive_topology},
    owned::Mesh0Owned,
    section::Mesh0SectionBodyOwned,
    section_kind,
    sections::*,
};

pub fn validate_mesh0_owned(mesh: &Mesh0Owned) -> AssetResult<()> {
    let mut seen = HashSet::new();
    for section in &mesh.sections {
        if !seen.insert((section.kind, section.key)) {
            return Err(AssetError::DuplicateSection {
                kind: section.kind,
                key: section.key,
            });
        }
    }

    let info = mesh
        .info()
        .ok_or(AssetError::MissingRequiredSection(section_kind::INFO, 0))?;
    let lod_count = mesh.lods().count();
    if lod_count == 0 {
        return Err(AssetError::InvalidData("mesh0 requires at least one LOD"));
    }
    if info.info.lod_count as usize != lod_count {
        return Err(AssetError::InvalidData("INFO lod_count mismatch"));
    }
    if !mesh
        .lod_section_keys()
        .any(|key| key == info.info.default_lod)
    {
        return Err(AssetError::InvalidData("INFO default_lod missing"));
    }

    let material_count = mesh
        .material_slots()
        .map(|slots| slots.slots.len())
        .unwrap_or_default();
    if info.info.material_slot_count as usize != material_count {
        return Err(AssetError::InvalidData("INFO material_slot_count mismatch"));
    }

    let source_features = mesh
        .sections
        .iter()
        .find_map(|section| match &section.body {
            Mesh0SectionBodyOwned::SourceFeatures(features) => Some(features.features.len()),
            _ => None,
        })
        .unwrap_or_default();
    if info.info.source_feature_count as usize != source_features {
        return Err(AssetError::InvalidData(
            "INFO source_feature_count mismatch",
        ));
    }

    let skinning_count = mesh
        .sections
        .iter()
        .filter(|section| matches!(section.body, Mesh0SectionBodyOwned::Skinning(_)))
        .count();
    if skinning_count > 1 {
        return Err(AssetError::InvalidData("too many skinning sections"));
    }
    if info.info.skinning_section_count as usize != skinning_count {
        return Err(AssetError::InvalidData(
            "INFO skinning_section_count mismatch",
        ));
    }
    if !valid_bounds(info.info.bounds_min, info.info.bounds_max) {
        return Err(AssetError::InvalidData("invalid mesh bounds"));
    }

    for lod in mesh.lods() {
        validate_lod_section(lod, info.info.material_slot_count)?;
    }
    Ok(())
}

pub fn validate_lod_section(
    lod: &Mesh0LodSectionOwned,
    material_slot_count: u32,
) -> AssetResult<()> {
    let header = &lod.header;
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
    if lod.vertex_bytes.len() != expected_vertices as usize {
        return Err(AssetError::InvalidData("invalid vertex buffer size"));
    }
    if lod.index_bytes.len() != expected_indices as usize {
        return Err(AssetError::InvalidData("invalid index buffer size"));
    }
    if !valid_bounds(header.bounds_min, header.bounds_max) {
        return Err(AssetError::InvalidData("invalid LOD bounds"));
    }

    for submesh in &lod.submeshes {
        checked_range_u32(
            submesh.vertex_start,
            submesh.vertex_count,
            header.vertex_count,
        )?;
        checked_range_u32(submesh.index_start, submesh.index_count, header.index_count)?;
        checked_range_u32(
            submesh.joint_palette_start,
            submesh.joint_palette_count,
            lod.joint_palette.len() as u32,
        )?;
        if submesh.material_slot >= material_slot_count {
            return Err(AssetError::InvalidData(
                "submesh material slot out of range",
            ));
        }
    }
    for batch in &lod.draw_batches {
        checked_range_u32(batch.vertex_start, batch.vertex_count, header.vertex_count)?;
        checked_range_u32(batch.index_start, batch.index_count, header.index_count)?;
        if batch.submesh_index >= lod.submeshes.len() as u32 {
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

fn checked_range_u32(start: u32, count: u32, len: u32) -> AssetResult<()> {
    if start.checked_add(count).filter(|end| *end <= len).is_none() {
        return Err(AssetError::RangeOutOfBounds);
    }
    Ok(())
}

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}
