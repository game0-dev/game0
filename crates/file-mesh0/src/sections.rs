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
    MaterialSlots(MaterialSlotsSectionView<R>),
    Skinning(SkinningSectionView<R>),
    SkeletonRefs(SkeletonRefsSectionView<R>),
    AnimationRefs(AnimationRefsSectionView<R>),
    EffectRefs(EffectRefsSectionView<R>),
    CollisionRefs(CollisionRefsSectionView<R>),
    AttachmentRefs(AttachmentRefsSectionView<R>),
    SourceFeatures(SourceFeaturesSectionView<R>),
    SourceDebug(SourceDebugSectionView<R>),
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
            section_type::MATERIAL_SLOTS => {
                Self::MaterialSlots(MaterialSlotsSectionView::new(reader, len))
            }
            section_type::SKINNING => Self::Skinning(SkinningSectionView::new(reader, len)),
            section_type::SKELETON_REFS => {
                Self::SkeletonRefs(SkeletonRefsSectionView::new(reader, len))
            }
            section_type::ANIMATION_REFS => {
                Self::AnimationRefs(AnimationRefsSectionView::new(reader, len))
            }
            section_type::EFFECT_REFS => Self::EffectRefs(EffectRefsSectionView::new(reader, len)),
            section_type::COLLISION_REFS => {
                Self::CollisionRefs(CollisionRefsSectionView::new(reader, len))
            }
            section_type::ATTACHMENT_REFS => {
                Self::AttachmentRefs(AttachmentRefsSectionView::new(reader, len))
            }
            section_type::SOURCE_FEATURES => {
                Self::SourceFeatures(SourceFeaturesSectionView::new(reader, len))
            }
            section_type::SOURCE_DEBUG => {
                Self::SourceDebug(SourceDebugSectionView::new(reader, len))
            }
            section_type::LOD => Self::Lod(LodSectionView::new(reader, len)),
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
            Self::MaterialSlots(section) => {
                SectionOwned::MaterialSlots(section.read_owned().await?)
            }
            Self::Skinning(section) => SectionOwned::Skinning(section.read_owned().await?),
            Self::SkeletonRefs(section) => SectionOwned::SkeletonRefs(section.read_owned().await?),
            Self::AnimationRefs(section) => {
                SectionOwned::AnimationRefs(section.read_owned().await?)
            }
            Self::EffectRefs(section) => SectionOwned::EffectRefs(section.read_owned().await?),
            Self::CollisionRefs(section) => {
                SectionOwned::CollisionRefs(section.read_owned().await?)
            }
            Self::AttachmentRefs(section) => {
                SectionOwned::AttachmentRefs(section.read_owned().await?)
            }
            Self::SourceFeatures(section) => {
                SectionOwned::SourceFeatures(section.read_owned().await?)
            }
            Self::SourceDebug(section) => SectionOwned::SourceDebug(section.read_owned().await?),
            Self::Lod(section) => SectionOwned::Lod(Box::new(section.read_owned().await?)),
        })
    }
}

pub enum SectionOwned {
    MeshInfo(MeshInfoSection),
    MaterialSlots(MaterialSlotsSectionOwned),
    Skinning(SkinningSectionOwned),
    SkeletonRefs(SkeletonRefsSectionOwned),
    AnimationRefs(AnimationRefsSectionOwned),
    EffectRefs(EffectRefsSectionOwned),
    CollisionRefs(CollisionRefsSectionOwned),
    AttachmentRefs(AttachmentRefsSectionOwned),
    SourceFeatures(SourceFeaturesSectionOwned),
    SourceDebug(SourceDebugSectionOwned),
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
