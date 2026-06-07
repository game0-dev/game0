use file_core::{AssetResult, SectionedAssetBuilder};
use bytes::Bytes;

use crate::{
    section::{Mesh0SectionBodyOwned, Mesh0SectionOwned},
    section_kind,
    sections::{Mesh0InfoSectionOwned, Mesh0LodSectionOwned, Mesh0MaterialSlotsSectionOwned},
    validate::validate_mesh0_owned,
};

pub struct Mesh0Owned {
    pub version: u32,
    pub sections: Vec<Mesh0SectionOwned>,
}

impl Mesh0Owned {
    pub fn validate(&self) -> AssetResult<()> {
        validate_mesh0_owned(self)
    }

    pub fn encode(&self) -> AssetResult<Bytes> {
        self.validate()?;
        let mut builder = SectionedAssetBuilder::new(self.version);
        for section in &self.sections {
            builder.add_section(section.encode_to_section_build()?)?;
        }
        builder.encode()
    }

    pub fn info(&self) -> Option<&Mesh0InfoSectionOwned> {
        self.sections
            .iter()
            .find_map(|section| match &section.body {
                Mesh0SectionBodyOwned::Info(info) => Some(info),
                _ => None,
            })
    }

    pub fn material_slots(&self) -> Option<&Mesh0MaterialSlotsSectionOwned> {
        self.sections
            .iter()
            .find_map(|section| match &section.body {
                Mesh0SectionBodyOwned::MaterialSlots(slots) => Some(slots),
                _ => None,
            })
    }

    pub fn lods(&self) -> impl Iterator<Item = &Mesh0LodSectionOwned> {
        self.sections
            .iter()
            .filter_map(|section| match &section.body {
                Mesh0SectionBodyOwned::Lod(lod) => Some(lod.as_ref()),
                _ => None,
            })
    }

    pub fn lod_section_keys(&self) -> impl Iterator<Item = u32> + '_ {
        self.sections
            .iter()
            .filter(|section| section.kind == section_kind::LOD)
            .map(|section| section.key)
    }
}
