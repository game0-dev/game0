use bytes::Bytes;
use file_core::{align::align_up, AssetError, AssetResult, EncodeBuffer};

use crate::{
    mesh0_view::{write_section_table_item, Mesh0Header, SECTION_TABLE_ITEM_BYTE_SIZE},
    sections::{validate_lod_section, SectionOwned},
};

pub struct Mesh0Owned {
    pub sections: Vec<SectionOwned>,
}

impl Mesh0Owned {
    pub fn get_sections_by_type(&self, section_type: u32) -> impl Iterator<Item = &SectionOwned> {
        self.sections
            .iter()
            .filter(move |section| section.section_type() == section_type)
    }

    pub fn write(&self) -> AssetResult<Bytes> {
        validate_mesh0_owned(self)?;
        let section_count = u32::try_from(self.sections.len())?;
        let table_size = self.sections.len() * SECTION_TABLE_ITEM_BYTE_SIZE;
        let body_start = align_up(Mesh0Header::BYTE_SIZE + table_size, 8)?;

        let mut out = EncodeBuffer::new();
        Mesh0Header::new(section_count).write(&mut out);

        let mut section_bodies = Vec::with_capacity(self.sections.len());
        let mut body_offset = body_start;
        for section in &self.sections {
            let section_type = section.section_type();
            let bytes = section.write()?;
            let offset = align_up(body_offset, 8)?;
            body_offset = offset
                .checked_add(bytes.len())
                .ok_or(file_core::AssetError::OffsetOverflow)?;

            write_section_table_item(
                &mut out,
                section_type,
                u32::try_from(offset)?,
                u32::try_from(bytes.len())?,
            );
            section_bodies.push((offset, bytes));
        }

        let mut out = out.into_inner();
        out.resize(body_start, 0);
        for (offset, bytes) in section_bodies {
            out.resize(offset, 0);
            out.extend_from_slice(&bytes);
        }
        Ok(Bytes::from(out))
    }
}

fn validate_mesh0_owned(mesh: &Mesh0Owned) -> AssetResult<()> {
    let material_count = mesh
        .get_sections_by_type(crate::sections::section_type::MATERIAL_SLOTS)
        .filter_map(|section| match section {
            SectionOwned::MaterialSlots(slots) => Some(slots.slots.len()),
            _ => None,
        })
        .sum::<usize>();
    let material_count = u32::try_from(material_count)?;

    for section in &mesh.sections {
        if let SectionOwned::MeshInfo(info) = section {
            if !valid_bounds(info.bounds_min, info.bounds_max) {
                return Err(AssetError::InvalidData("invalid mesh bounds"));
            }
        }
    }

    for section in mesh.get_sections_by_type(crate::sections::section_type::LOD) {
        if let SectionOwned::Lod(lod) = section {
            validate_lod_section(lod, material_count)?;
        }
    }
    Ok(())
}

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}
