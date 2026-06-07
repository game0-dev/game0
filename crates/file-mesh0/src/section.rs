use file_core::{AssetResult, SectionBuild};
use bytes::Bytes;

use crate::{
    encode::{
        encode_info_section, encode_lod_section, encode_material_slots_section,
        encode_raw_asset_refs, encode_skinning_section, encode_source_features_section,
    },
    sections::*,
};

pub struct Mesh0SectionOwned {
    pub kind: u32,
    pub key: u32,
    pub flags: u32,
    pub extra: u32,
    pub body: Mesh0SectionBodyOwned,
}

pub enum Mesh0SectionBodyOwned {
    Info(Mesh0InfoSectionOwned),
    MaterialSlots(Mesh0MaterialSlotsSectionOwned),
    Skinning(Mesh0SkinningSectionOwned),
    SkeletonRefs(Vec<Mesh0AssetRef>),
    AnimationRefs(Vec<Mesh0AssetRef>),
    EffectRefs(Vec<Mesh0AssetRef>),
    CollisionRefs(Vec<Mesh0AssetRef>),
    AttachmentRefs(Vec<Mesh0AssetRef>),
    SourceFeatures(Mesh0SourceFeaturesSectionOwned),
    SourceDebug(Bytes),
    Lod(Box<Mesh0LodSectionOwned>),
    Raw(Bytes),
}

pub trait Mesh0SectionBodyEncode {
    fn encode_body(&self) -> AssetResult<Bytes>;
}

impl Mesh0SectionBodyEncode for Mesh0SectionBodyOwned {
    fn encode_body(&self) -> AssetResult<Bytes> {
        match self {
            Self::Info(section) => encode_info_section(section),
            Self::MaterialSlots(section) => encode_material_slots_section(section),
            Self::Skinning(section) => encode_skinning_section(section),
            Self::SkeletonRefs(refs)
            | Self::AnimationRefs(refs)
            | Self::EffectRefs(refs)
            | Self::CollisionRefs(refs)
            | Self::AttachmentRefs(refs) => encode_raw_asset_refs(refs),
            Self::SourceFeatures(section) => encode_source_features_section(section),
            Self::SourceDebug(bytes) | Self::Raw(bytes) => Ok(bytes.clone()),
            Self::Lod(section) => encode_lod_section(section),
        }
    }
}

impl Mesh0SectionOwned {
    pub fn encode_to_section_build(&self) -> AssetResult<SectionBuild> {
        Ok(SectionBuild {
            kind: self.kind,
            key: self.key,
            flags: self.flags,
            extra: self.extra,
            bytes: self.body.encode_body()?,
        })
    }
}
