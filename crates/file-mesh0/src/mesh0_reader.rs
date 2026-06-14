use bytes::Bytes;
use file_core::{AssetError, AssetRead, AssetResult, DecodeCursor};
use tokio::sync::OnceCell;

pub const MESH0_HEADER_BYTE_SIZE: usize = 8;
pub const SECTION_TABLE_ITEM_BYTE_SIZE: usize = 12;

use crate::sections::{
    AnimationRefsSection, AttachmentRefsSection, CollisionRefsSection, EffectRefsSection,
    MaterialSlotsSection, MeshInfoReader, RenderVariantReader, SkeletonRefsSection,
    SkinningSection, SourceDebugSection, SourceFeaturesSection,
};
use crate::{section_type, MESH0_VERSION_0};

#[derive(Clone)]
pub struct SectionEntry<S> {
    pub section_type: u32,
    pub offset: u32,
    pub len: u32,
    pub section: OnceCell<S>,
}

#[derive(Clone)]
pub struct Mesh0Reader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: R,
    mesh_info: SectionEntry<MeshInfoReader<R>>,
    render_variants: Vec<SectionEntry<RenderVariantReader<R>>>,
    material_slots: Option<SectionEntry<MaterialSlotsSection>>,
    skinning: Option<SectionEntry<SkinningSection>>,
    skeleton_refs: Option<SectionEntry<SkeletonRefsSection>>,
    animation_refs: Option<SectionEntry<AnimationRefsSection>>,
    effect_refs: Option<SectionEntry<EffectRefsSection>>,
    collision_refs: Option<SectionEntry<CollisionRefsSection>>,
    attachment_refs: Option<SectionEntry<AttachmentRefsSection>>,
    source_features: Option<SectionEntry<SourceFeaturesSection>>,
    source_debug: Option<SectionEntry<SourceDebugSection>>,
}

impl<R> Mesh0Reader<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn open(reader: R) -> AssetResult<Self> {
        let header_bytes = reader.read_at(0, MESH0_HEADER_BYTE_SIZE as u64).await?;
        let mut header = DecodeCursor::new(&header_bytes);
        let version = header.read_u32_le()?;
        if version != MESH0_VERSION_0 {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        let section_count = header.read_u32_le()?;
        let table_len = u64::from(section_count)
            .checked_mul(SECTION_TABLE_ITEM_BYTE_SIZE as u64)
            .ok_or(AssetError::OffsetOverflow)?;
        let table_bytes = reader
            .read_at(MESH0_HEADER_BYTE_SIZE as u64, table_len)
            .await?;
        let mut table = DecodeCursor::new(&table_bytes);

        let mut mesh_info = None;
        let mut material_slots = None;
        let mut skinning = None;
        let mut skeleton_refs = None;
        let mut animation_refs = None;
        let mut effect_refs = None;
        let mut collision_refs = None;
        let mut attachment_refs = None;
        let mut source_features = None;
        let mut source_debug = None;
        let mut render_variants = Vec::new();

        for _ in 0..section_count {
            let raw = RawSectionEntry {
                section_type: table.read_u32_le()?,
                offset: table.read_u32_le()?,
                len: table.read_u32_le()?,
            };
            match raw.section_type {
                section_type::MESH_INFO => set_once(&mut mesh_info, raw)?,
                section_type::MATERIAL_SLOTS => set_once(&mut material_slots, raw)?,
                section_type::SKINNING => set_once(&mut skinning, raw)?,
                section_type::SKELETON_REFS => set_once(&mut skeleton_refs, raw)?,
                section_type::ANIMATION_REFS => set_once(&mut animation_refs, raw)?,
                section_type::EFFECT_REFS => set_once(&mut effect_refs, raw)?,
                section_type::COLLISION_REFS => set_once(&mut collision_refs, raw)?,
                section_type::ATTACHMENT_REFS => set_once(&mut attachment_refs, raw)?,
                section_type::SOURCE_FEATURES => set_once(&mut source_features, raw)?,
                section_type::SOURCE_DEBUG => set_once(&mut source_debug, raw)?,
                section_type::RENDER_VARIANT => render_variants.push(entry(raw)),
                _ => return Err(AssetError::InvalidData("invalid section type")),
            }
        }

        Ok(Self {
            reader,
            mesh_info: mesh_info.ok_or(AssetError::InvalidData("missing mesh info section"))?,
            render_variants,
            material_slots,
            skinning,
            skeleton_refs,
            animation_refs,
            effect_refs,
            collision_refs,
            attachment_refs,
            source_features,
            source_debug,
        })
    }

    pub async fn mesh_info(&self) -> AssetResult<&MeshInfoReader<R>> {
        self.mesh_info
            .section
            .get_or_try_init(|| async {
                MeshInfoReader::read(
                    self.reader.clone(),
                    u64::from(self.mesh_info.offset),
                    self.mesh_info.len,
                )
                .await
            })
            .await
    }

    pub async fn material_slots(&self) -> AssetResult<Option<&MaterialSlotsSection>> {
        self.read_optional_body_section(&self.material_slots, MaterialSlotsSection::read)
            .await
    }

    pub async fn skinning(&self) -> AssetResult<Option<&SkinningSection>> {
        self.read_optional_body_section(&self.skinning, SkinningSection::read)
            .await
    }

    pub async fn skeleton_refs(&self) -> AssetResult<Option<&SkeletonRefsSection>> {
        self.read_optional_body_section(&self.skeleton_refs, SkeletonRefsSection::read)
            .await
    }

    pub async fn animation_refs(&self) -> AssetResult<Option<&AnimationRefsSection>> {
        self.read_optional_body_section(&self.animation_refs, AnimationRefsSection::read)
            .await
    }

    pub async fn effect_refs(&self) -> AssetResult<Option<&EffectRefsSection>> {
        self.read_optional_body_section(&self.effect_refs, EffectRefsSection::read)
            .await
    }

    pub async fn collision_refs(&self) -> AssetResult<Option<&CollisionRefsSection>> {
        self.read_optional_body_section(&self.collision_refs, CollisionRefsSection::read)
            .await
    }

    pub async fn attachment_refs(&self) -> AssetResult<Option<&AttachmentRefsSection>> {
        self.read_optional_body_section(&self.attachment_refs, AttachmentRefsSection::read)
            .await
    }

    pub async fn source_features(&self) -> AssetResult<Option<&SourceFeaturesSection>> {
        self.read_optional_body_section(&self.source_features, SourceFeaturesSection::read)
            .await
    }

    pub async fn source_debug(&self) -> AssetResult<Option<&SourceDebugSection>> {
        self.read_optional_body_section(&self.source_debug, SourceDebugSection::read)
            .await
    }

    pub async fn render_variants(&self) -> AssetResult<Vec<&RenderVariantReader<R>>> {
        let mut variants = Vec::with_capacity(self.render_variants.len());
        for index in 0..self.render_variants.len() {
            variants.push(self.render_variant(index).await?);
        }
        Ok(variants)
    }

    pub async fn render_variant(&self, index: usize) -> AssetResult<&RenderVariantReader<R>> {
        let entry = self
            .render_variants
            .get(index)
            .ok_or(AssetError::RangeOutOfBounds)?;
        entry
            .section
            .get_or_try_init(|| async {
                RenderVariantReader::read(self.reader.clone(), u64::from(entry.offset), entry.len)
                    .await
            })
            .await
    }

    async fn read_optional_body_section<'a, S>(
        &self,
        entry: &'a Option<SectionEntry<S>>,
        read: fn(Bytes) -> AssetResult<S>,
    ) -> AssetResult<Option<&'a S>> {
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry
            .section
            .get_or_try_init(|| async {
                let bytes = self
                    .reader
                    .read_at(u64::from(entry.offset), u64::from(entry.len))
                    .await?;
                read(bytes)
            })
            .await
            .map(Some)
    }
}

#[derive(Clone, Copy)]
struct RawSectionEntry {
    section_type: u32,
    offset: u32,
    len: u32,
}

fn entry<S>(raw: RawSectionEntry) -> SectionEntry<S> {
    SectionEntry {
        section_type: raw.section_type,
        offset: raw.offset,
        len: raw.len,
        section: OnceCell::new(),
    }
}

fn set_once<S>(slot: &mut Option<SectionEntry<S>>, raw: RawSectionEntry) -> AssetResult<()> {
    if slot.is_some() {
        return Err(AssetError::InvalidData("duplicate section"));
    }
    *slot = Some(entry(raw));
    Ok(())
}
