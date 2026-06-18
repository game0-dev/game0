use std::{fs, path::Path};

use bytes::Bytes;
use file_core::{align::align_up, AssetError, EncodeBuffer};

use crate::mesh0_reader::{Mesh0Header, SectionEntry};
use crate::sections::{
    AnimationReader, MeshInfoBuilder, RenderVariantBuilder, SkeletonReader, ANIMATION, MESH_INFO,
    RENDER_VARIANT, SKELETON,
};
use crate::MESH0_VERSION;

struct SectionBody {
    section_type: u32,
    file_id: u32,
    bytes: Bytes,
}

pub struct Mesh0Builder {
    pub mesh_info: MeshInfoBuilder,
    pub skeleton: Option<SkeletonReader>,
    pub animation: Vec<AnimationReader>,
    pub render_variants: Vec<RenderVariantBuilder>,
}

impl Mesh0Builder {
    pub fn new(mesh_info: MeshInfoBuilder) -> Self {
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
        self.mesh_info.validate()?;
        if self.render_variants.is_empty() {
            return Err(AssetError::InvalidData("missing render variant section"));
        }
        for variant in &self.render_variants {
            variant.validate(&self.mesh_info.header)?;
        }
        Ok(())
    }

    pub fn write_bytes(&self) -> file_core::AssetResult<Bytes> {
        self.validate()?;
        let sections = self.section_bodies()?;
        let section_count = u32::try_from(sections.len())?;
        let table_size = sections.len() * SectionEntry::<()>::BYTE_SIZE;
        let body_start = align_up(Mesh0Header::BYTE_SIZE + table_size, 8)?;

        let mut out = EncodeBuffer::new();
        out.write_u32_le(MESH0_VERSION);
        out.write_u32_le(section_count);

        let mut section_bodies = Vec::with_capacity(sections.len());
        let mut body_offset = body_start;
        for section in sections {
            let (offset, len) = if section.file_id == 0 {
                let offset = align_up(body_offset, 8)?;
                body_offset = offset
                    .checked_add(section.bytes.len())
                    .ok_or(AssetError::OffsetOverflow)?;
                let len = u32::try_from(section.bytes.len())?;
                section_bodies.push((offset, section.bytes));
                (u32::try_from(offset)?, len)
            } else {
                (0, 0)
            };
            out.write_u32_le(section.section_type);
            out.write_u32_le(section.file_id);
            out.write_u32_le(offset);
            out.write_u32_le(len);
        }

        let mut bytes = out.into_inner();
        bytes.resize(body_start, 0);
        for (offset, body) in section_bodies {
            bytes.resize(offset, 0);
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
        sections.push(inline_section(MESH_INFO, self.mesh_info.write()?));
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
