pub mod animation_refs;
pub mod attachment_refs;
pub mod collision_refs;
pub mod effect_refs;
pub mod lod;
pub mod material_slots;
pub mod mesh_info;
pub mod skeleton_refs;
pub mod skinning;
pub mod source_debug;
pub mod source_features;

use bytes::Bytes;
use file_core::{AssetError, AssetRead, AssetResult, OffsetAssetReader};

pub use animation_refs::*;
pub use attachment_refs::*;
pub use collision_refs::*;
pub use effect_refs::*;
pub use lod::*;
pub use material_slots::*;
pub use mesh_info::*;
pub use skeleton_refs::*;
pub use skinning::*;
pub use source_debug::*;
pub use source_features::*;

pub const MESH0_VERSION_0: u32 = 0;

pub mod section_type {
    pub const MESH_INFO: u32 = 1;
    pub const LOD: u32 = 11;
    pub const MATERIAL_SLOTS: u32 = 2;
    pub const SKINNING: u32 = 3;
    pub const SKELETON_REFS: u32 = 4;
    pub const ANIMATION_REFS: u32 = 5;
    pub const EFFECT_REFS: u32 = 6;
    pub const COLLISION_REFS: u32 = 7;
    pub const ATTACHMENT_REFS: u32 = 8;
    pub const SOURCE_FEATURES: u32 = 9;
    pub const SOURCE_DEBUG: u32 = 10;
}

#[derive(Clone)]
pub enum SectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    MeshInfo(MeshInfoSection),
    MaterialSlots(MaterialSlotsSection),
    Skinning(SkinningSection),
    SkeletonRefs(SkeletonRefsSection),
    AnimationRefs(AnimationRefsSection),
    EffectRefs(EffectRefsSection),
    CollisionRefs(CollisionRefsSection),
    AttachmentRefs(AttachmentRefsSection),
    SourceFeatures(SourceFeaturesSection),
    SourceDebug(SourceDebugSection),
    Lod(LodSectionView<R>),
}

impl<R> SectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub(crate) async fn read(
        reader: OffsetAssetReader<R>,
        section_type: u32,
        len: u32,
    ) -> AssetResult<Self> {
        Ok(match section_type {
            section_type::MESH_INFO => Self::MeshInfo(MeshInfoSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::LOD => Self::Lod(LodSectionView::read(reader, len).await?),
            section_type::MATERIAL_SLOTS => Self::MaterialSlots(MaterialSlotsSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::SKINNING => Self::Skinning(SkinningSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::SKELETON_REFS => Self::SkeletonRefs(SkeletonRefsSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::ANIMATION_REFS => Self::AnimationRefs(AnimationRefsSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::EFFECT_REFS => Self::EffectRefs(EffectRefsSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::COLLISION_REFS => Self::CollisionRefs(CollisionRefsSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::ATTACHMENT_REFS => Self::AttachmentRefs(AttachmentRefsSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::SOURCE_FEATURES => Self::SourceFeatures(SourceFeaturesSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            section_type::SOURCE_DEBUG => Self::SourceDebug(SourceDebugSection::read(
                reader.read_at(0, u64::from(len)).await?,
            )?),
            _ => return Err(AssetError::InvalidSectionType(section_type)),
        })
    }

    pub fn section_type(&self) -> u32 {
        match self {
            Self::MeshInfo(_) => section_type::MESH_INFO,
            Self::MaterialSlots(_) => section_type::MATERIAL_SLOTS,
            Self::Skinning(_) => section_type::SKINNING,
            Self::SkeletonRefs(_) => section_type::SKELETON_REFS,
            Self::AnimationRefs(_) => section_type::ANIMATION_REFS,
            Self::EffectRefs(_) => section_type::EFFECT_REFS,
            Self::CollisionRefs(_) => section_type::COLLISION_REFS,
            Self::AttachmentRefs(_) => section_type::ATTACHMENT_REFS,
            Self::SourceFeatures(_) => section_type::SOURCE_FEATURES,
            Self::SourceDebug(_) => section_type::SOURCE_DEBUG,
            Self::Lod(_) => section_type::LOD,
        }
    }

    pub async fn read_owned(&self) -> AssetResult<SectionOwned> {
        Ok(match self {
            Self::MeshInfo(section) => SectionOwned::MeshInfo(section.clone()),
            Self::MaterialSlots(section) => SectionOwned::MaterialSlots(section.clone()),
            Self::Skinning(section) => SectionOwned::Skinning(section.clone()),
            Self::SkeletonRefs(section) => SectionOwned::SkeletonRefs(section.clone()),
            Self::AnimationRefs(section) => SectionOwned::AnimationRefs(section.clone()),
            Self::EffectRefs(section) => SectionOwned::EffectRefs(section.clone()),
            Self::CollisionRefs(section) => SectionOwned::CollisionRefs(section.clone()),
            Self::AttachmentRefs(section) => SectionOwned::AttachmentRefs(section.clone()),
            Self::SourceFeatures(section) => SectionOwned::SourceFeatures(section.clone()),
            Self::SourceDebug(section) => SectionOwned::SourceDebug(section.clone()),
            Self::Lod(section) => SectionOwned::Lod(Box::new(section.read_owned().await?)),
        })
    }
}

pub enum SectionOwned {
    MeshInfo(MeshInfoSection),
    MaterialSlots(MaterialSlotsSection),
    Skinning(SkinningSection),
    SkeletonRefs(SkeletonRefsSection),
    AnimationRefs(AnimationRefsSection),
    EffectRefs(EffectRefsSection),
    CollisionRefs(CollisionRefsSection),
    AttachmentRefs(AttachmentRefsSection),
    SourceFeatures(SourceFeaturesSection),
    SourceDebug(SourceDebugSection),
    Lod(Box<LodSectionOwned>),
}

impl SectionOwned {
    pub fn section_type(&self) -> u32 {
        match self {
            Self::MeshInfo(_) => section_type::MESH_INFO,
            Self::MaterialSlots(_) => section_type::MATERIAL_SLOTS,
            Self::Skinning(_) => section_type::SKINNING,
            Self::SkeletonRefs(_) => section_type::SKELETON_REFS,
            Self::AnimationRefs(_) => section_type::ANIMATION_REFS,
            Self::EffectRefs(_) => section_type::EFFECT_REFS,
            Self::CollisionRefs(_) => section_type::COLLISION_REFS,
            Self::AttachmentRefs(_) => section_type::ATTACHMENT_REFS,
            Self::SourceFeatures(_) => section_type::SOURCE_FEATURES,
            Self::SourceDebug(_) => section_type::SOURCE_DEBUG,
            Self::Lod(_) => section_type::LOD,
        }
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        match self {
            Self::MeshInfo(section) => section.write(),
            Self::MaterialSlots(section) => section.write(),
            Self::Skinning(section) => section.write(),
            Self::SkeletonRefs(section) => section.write(),
            Self::AnimationRefs(section) => section.write(),
            Self::EffectRefs(section) => section.write(),
            Self::CollisionRefs(section) => section.write(),
            Self::AttachmentRefs(section) => section.write(),
            Self::SourceFeatures(section) => section.write(),
            Self::SourceDebug(section) => section.write(),
            Self::Lod(section) => section.write(),
        }
    }
}
