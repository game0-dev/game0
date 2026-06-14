use std::{fs, path::Path};

use bytes::Bytes;
use file_core::{align::align_up, AssetError, EncodeBuffer};

use crate::sections::{
    AnimationRefsSection, AttachmentRefsSection, CollisionRefsSection, EffectRefsSection,
    MaterialSlotsSection, MeshInfoBuilder, RenderVariantBuilder, SkeletonRefsSection,
    SkinningSection, SourceDebugSection, SourceFeaturesSection,
};
use crate::{section_type, MESH0_VERSION_0};

pub struct Mesh0Builder {
    pub mesh_info: MeshInfoBuilder,
    pub material_slots: Option<MaterialSlotsSection>,
    pub skinning: Option<SkinningSection>,
    pub skeleton_refs: Option<SkeletonRefsSection>,
    pub animation_refs: Option<AnimationRefsSection>,
    pub effect_refs: Option<EffectRefsSection>,
    pub collision_refs: Option<CollisionRefsSection>,
    pub attachment_refs: Option<AttachmentRefsSection>,
    pub source_features: Option<SourceFeaturesSection>,
    pub source_debug: Option<SourceDebugSection>,
    pub render_variants: Vec<RenderVariantBuilder>,
}

impl Mesh0Builder {
    pub fn new(mesh_info: MeshInfoBuilder) -> Self {
        Self {
            mesh_info,
            material_slots: None,
            skinning: None,
            skeleton_refs: None,
            animation_refs: None,
            effect_refs: None,
            collision_refs: None,
            attachment_refs: None,
            source_features: None,
            source_debug: None,
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
        let material_slot_count = self
            .material_slots
            .as_ref()
            .map(|slots| slots.slots.len())
            .unwrap_or(0);
        let material_slot_count = u32::try_from(material_slot_count)?;
        for variant in &self.render_variants {
            variant.validate(&self.mesh_info.header, material_slot_count)?;
        }
        Ok(())
    }

    pub fn write_bytes(&self) -> file_core::AssetResult<Bytes> {
        self.validate()?;
        let sections = self.section_bodies()?;
        let section_count = u32::try_from(sections.len())?;
        let table_size = sections.len() * crate::SECTION_TABLE_ITEM_BYTE_SIZE;
        let body_start = align_up(crate::MESH0_HEADER_BYTE_SIZE + table_size, 8)?;

        let mut out = EncodeBuffer::new();
        out.write_u32_le(MESH0_VERSION_0);
        out.write_u32_le(section_count);

        let mut section_bodies = Vec::with_capacity(sections.len());
        let mut body_offset = body_start;
        for (section_type, bytes) in sections {
            let offset = align_up(body_offset, 8)?;
            body_offset = offset
                .checked_add(bytes.len())
                .ok_or(AssetError::OffsetOverflow)?;
            out.write_u32_le(section_type);
            out.write_u32_le(u32::try_from(offset)?);
            out.write_u32_le(u32::try_from(bytes.len())?);
            section_bodies.push((offset, bytes));
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

    fn section_bodies(&self) -> file_core::AssetResult<Vec<(u32, Bytes)>> {
        let mut sections = Vec::new();
        sections.push((section_type::MESH_INFO, self.mesh_info.write()?));
        if let Some(section) = &self.material_slots {
            sections.push((section_type::MATERIAL_SLOTS, section.write()?));
        }
        if let Some(section) = &self.skinning {
            sections.push((section_type::SKINNING, section.write()?));
        }
        if let Some(section) = &self.skeleton_refs {
            sections.push((section_type::SKELETON_REFS, section.write()?));
        }
        if let Some(section) = &self.animation_refs {
            sections.push((section_type::ANIMATION_REFS, section.write()?));
        }
        if let Some(section) = &self.effect_refs {
            sections.push((section_type::EFFECT_REFS, section.write()?));
        }
        if let Some(section) = &self.collision_refs {
            sections.push((section_type::COLLISION_REFS, section.write()?));
        }
        if let Some(section) = &self.attachment_refs {
            sections.push((section_type::ATTACHMENT_REFS, section.write()?));
        }
        if let Some(section) = &self.source_features {
            sections.push((section_type::SOURCE_FEATURES, section.write()?));
        }
        if let Some(section) = &self.source_debug {
            sections.push((section_type::SOURCE_DEBUG, section.write()?));
        }
        for variant in &self.render_variants {
            sections.push((section_type::RENDER_VARIANT, variant.write()?));
        }
        Ok(sections)
    }
}
