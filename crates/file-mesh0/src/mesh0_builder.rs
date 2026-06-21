use std::{fs, path::Path};

use bytes::Bytes;
use file_core::{AssetError, EncodeBuffer};

use crate::mesh0_reader::SectionEntry;
use crate::sections::{
    AnimationReader, MeshInfoHeader, RenderVariantBuilder, SkeletonReader, ANIMATION, MESH_INFO,
    RENDER_VARIANT, SKELETON,
};
use crate::MESH0_VERSION;

const HEADER_BYTE_SIZE: u64 = 8;

struct SectionBody {
    section_type: u32,
    file_id: u32,
    bytes: Bytes,
}

pub struct Mesh0Builder {
    pub mesh_info: MeshInfoHeader,
    pub skeleton: Option<SkeletonReader>,
    pub animation: Vec<AnimationReader>,
    pub render_variants: Vec<RenderVariantBuilder>,
}

impl Mesh0Builder {
    pub fn new(mesh_info: MeshInfoHeader) -> Self {
        Self {
            mesh_info,
            skeleton: None,
            animation: Vec::new(),
            render_variants: Vec::new(),
        }
    }

    pub fn push_render_variant(&mut self, variant: RenderVariantBuilder) {
        self.render_variants.push(variant);
    }

    pub fn validate(&self) -> file_core::AssetResult<()> {
        if self.mesh_info.bounding_box_min[0] > self.mesh_info.bounding_box_max[0]
            || self.mesh_info.bounding_box_min[1] > self.mesh_info.bounding_box_max[1]
            || self.mesh_info.bounding_box_min[2] > self.mesh_info.bounding_box_max[2]
        {
            return Err(AssetError::InvalidData("invalid mesh bounds"));
        }
        if self.render_variants.is_empty() {
            return Err(AssetError::InvalidData("missing render variant section"));
        }
        for variant in &self.render_variants {
            variant.validate(&self.mesh_info)?;
        }
        Ok(())
    }

    pub fn write_bytes(&self) -> file_core::AssetResult<Bytes> {
        self.validate()?;
        let sections = self.section_bodies()?;
        let section_count = u32::try_from(sections.len())?;
        let table_size = u64::try_from(sections.len())?
            .checked_mul(SectionEntry::<()>::BYTE_SIZE)
            .ok_or(AssetError::OffsetOverflow)?;
        let body_start = align_up_u64(
            HEADER_BYTE_SIZE
                .checked_add(table_size)
                .ok_or(AssetError::OffsetOverflow)?,
            8,
        )?;

        let mut out = EncodeBuffer::new();
        out.write_u32_le(MESH0_VERSION);
        out.write_u32_le(section_count);

        let mut section_bodies = Vec::with_capacity(sections.len());
        let mut body_offset = body_start;
        for section in sections {
            let (offset, len) = if section.file_id == 0 {
                let offset = align_up_u64(body_offset, 8)?;
                let body_len = u64::try_from(section.bytes.len())?;
                body_offset = offset
                    .checked_add(body_len)
                    .ok_or(AssetError::OffsetOverflow)?;
                section_bodies.push((offset, section.bytes));
                (u32::try_from(offset)?, u32::try_from(body_len)?)
            } else {
                (0, 0)
            };
            out.write_u32_le(section.section_type);
            out.write_u32_le(section.file_id);
            out.write_u32_le(offset);
            out.write_u32_le(len);
        }

        let mut bytes = out.into_inner();
        resize_vec(&mut bytes, body_start)?;
        for (offset, body) in section_bodies {
            resize_vec(&mut bytes, offset)?;
            bytes.extend_from_slice(&body);
        }
        Ok(Bytes::from(bytes))
    }

    pub fn write_file(&self, path: impl AsRef<Path>) -> file_core::AssetResult<()> {
        let path = path.as_ref();
        let bytes = self.write_bytes()?;
        fs::write(path, bytes).map_err(|error| AssetError::Io(error.to_string()))
    }

    fn section_bodies(&self) -> file_core::AssetResult<Vec<SectionBody>> {
        let mut sections = Vec::new();
        let mut mesh_info = EncodeBuffer::new();
        self.mesh_info.write(&mut mesh_info);
        sections.push(inline_section(
            MESH_INFO,
            Bytes::from(mesh_info.into_inner()),
        ));
        if let Some(section) = &self.skeleton {
            sections.push(inline_section(SKELETON, section.write()?));
        }
        for section in &self.animation {
            sections.push(inline_section(ANIMATION, section.write()?));
        }
        for variant in &self.render_variants {
            sections.push(inline_section(RENDER_VARIANT, variant.write()?));
        }
        Ok(sections)
    }
}

fn inline_section(section_type: u32, bytes: Bytes) -> SectionBody {
    SectionBody {
        section_type,
        file_id: 0,
        bytes,
    }
}

fn align_up_u64(value: u64, align: u64) -> file_core::AssetResult<u64> {
    if align <= 1 {
        return Ok(value);
    }
    let padded = value
        .checked_add(align.checked_sub(1).ok_or(AssetError::OffsetOverflow)?)
        .ok_or(AssetError::OffsetOverflow)?;
    Ok(padded / align * align)
}

fn resize_vec(bytes: &mut Vec<u8>, len: u64) -> file_core::AssetResult<()> {
    bytes.resize(usize::try_from(len)?, 0);
    Ok(())
}
