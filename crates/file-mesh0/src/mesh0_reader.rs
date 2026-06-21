use std::future::Future;

use file_core::{AssetError, AssetReader, AssetResult, DecodeCursor};
use tokio::sync::OnceCell;

use crate::sections::{
    AnimationReader, MeshInfoHeader, RenderVariantReader, SkeletonReader, ANIMATION, MESH_INFO,
    RENDER_VARIANT, SKELETON,
};
use crate::MESH0_VERSION;

#[derive(Clone)]
pub struct SectionEntry<S> {
    pub section_type: u32,
    pub file_id: u32,
    pub offset: u32,
    pub len: u32,
    pub section: OnceCell<S>,
}

impl<S> SectionEntry<S> {
    pub(crate) const BYTE_SIZE: u64 = 16;

    fn new(section_type: u32, file_id: u32, offset: u32, len: u32) -> Self {
        Self {
            section_type,
            file_id,
            offset,
            len,
            section: OnceCell::new(),
        }
    }

    async fn read<R, F, Fut>(&self, reader: &R, read_section: F) -> AssetResult<&S>
    where
        R: AssetReader,
        F: FnOnce(R) -> Fut,
        Fut: Future<Output = AssetResult<S>>,
    {
        self.section
            .get_or_try_init(|| async {
                let reader = if self.file_id != 0 {
                    reader.with_file(self.file_id)?
                } else {
                    reader.with_offset_accumulate(u64::from(self.offset))?
                };

                read_section(reader).await
            })
            .await
    }
}

#[derive(Clone)]
pub struct Mesh0Reader<R>
where
    R: AssetReader,
{
    reader: R,
    mesh_info: Option<SectionEntry<MeshInfoHeader>>,
    render_variants: Vec<SectionEntry<RenderVariantReader<R>>>,
    skeleton: Option<SectionEntry<SkeletonReader>>,
    animation: Vec<SectionEntry<AnimationReader>>,
}

impl<R> Mesh0Reader<R>
where
    R: AssetReader,
{
    pub(crate) const HEADER_BYTE_SIZE: u64 = 8;

    pub async fn open(reader: R) -> AssetResult<Self> {
        let mut header = DecodeCursor::from_reader(&reader, Self::HEADER_BYTE_SIZE).await?;
        let version = header.read_u32_le()?;
        if version != MESH0_VERSION {
            return Err(AssetError::UnsupportedFormatVersion(version));
        }
        let section_count = header.read_u32_le()?;
        let table_len = u64::from(section_count)
            .checked_mul(SectionEntry::<()>::BYTE_SIZE)
            .ok_or(AssetError::OffsetOverflow)?;

        let mut table =
            DecodeCursor::from_reader_at(&reader, Self::HEADER_BYTE_SIZE, table_len).await?;

        let mut mesh = Self {
            reader,
            mesh_info: None,
            render_variants: Vec::new(),
            skeleton: None,
            animation: Vec::new(),
        };

        for _ in 0..section_count {
            let section_type = table.read_u32_le()?;
            let file_id = table.read_u32_le()?;
            let offset = table.read_u32_le()?;
            let len = table.read_u32_le()?;
            match section_type {
                MESH_INFO => {
                    if mesh.mesh_info.is_some() {
                        return Err(AssetError::InvalidData("duplicate section"));
                    }
                    mesh.mesh_info = Some(SectionEntry::new(section_type, file_id, offset, len));
                }
                RENDER_VARIANT => {
                    mesh.render_variants.push(SectionEntry::new(
                        section_type,
                        file_id,
                        offset,
                        len,
                    ));
                }
                SKELETON => {
                    if mesh.skeleton.is_some() {
                        return Err(AssetError::InvalidData("duplicate section"));
                    }
                    mesh.skeleton = Some(SectionEntry::new(section_type, file_id, offset, len));
                }
                ANIMATION => {
                    mesh.animation
                        .push(SectionEntry::new(section_type, file_id, offset, len));
                }
                _ => return Err(AssetError::InvalidData("invalid section type")),
            }
        }

        Ok(mesh)
    }

    pub async fn read_mesh_info(&self) -> AssetResult<&MeshInfoHeader> {
        let entry = self
            .mesh_info
            .as_ref()
            .ok_or(AssetError::InvalidData("missing mesh info section"))?;

        entry.read(&self.reader, MeshInfoHeader::read).await
    }

    pub async fn read_skeleton(&self) -> AssetResult<Option<&SkeletonReader>> {
        let Some(entry) = &self.skeleton else {
            return Ok(None);
        };
        entry
            .read(&self.reader, SkeletonReader::read)
            .await
            .map(Some)
    }

    pub async fn read_render_variant(&self, index: usize) -> AssetResult<&RenderVariantReader<R>> {
        let entry = self
            .render_variants
            .get(index)
            .ok_or(AssetError::RangeOutOfBounds)?;

        entry.read(&self.reader, RenderVariantReader::read).await
    }

    pub async fn read_render_variants(&self) -> AssetResult<Vec<&RenderVariantReader<R>>> {
        let mut variants = Vec::with_capacity(self.render_variants.len());
        for index in 0..self.render_variants.len() {
            variants.push(self.read_render_variant(index).await?);
        }
        Ok(variants)
    }

    pub async fn read_animation(&self, index: usize) -> AssetResult<&AnimationReader> {
        let entry = self
            .animation
            .get(index)
            .ok_or(AssetError::RangeOutOfBounds)?;

        entry.read(&self.reader, AnimationReader::read).await
    }

    pub async fn read_animations(&self) -> AssetResult<Vec<&AnimationReader>> {
        let mut animations = Vec::with_capacity(self.animation.len());
        for index in 0..self.animation.len() {
            animations.push(self.read_animation(index).await?);
        }
        Ok(animations)
    }
}
