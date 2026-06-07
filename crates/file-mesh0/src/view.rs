use file_core::{
    AssetError, AssetRead, AssetResult, SectionRecord, SectionTable, SectionView,
    SectionedAssetHeader, SectionedAssetView,
};
use bytes::Bytes;
use tokio::sync::OnceCell;

use crate::{
    decode::{
        decode_asset_refs, decode_info_section, decode_lod_header, decode_material_slots_section,
        decode_skinning_section, decode_source_features_section, decode_table,
    },
    format::MESH0_VERSION_0,
    owned::Mesh0Owned,
    section::{Mesh0SectionBodyOwned, Mesh0SectionOwned},
    section_kind,
    sections::*,
};

#[derive(Clone)]
pub struct Mesh0View<R> {
    asset: SectionedAssetView<R>,
}

impl<R> Mesh0View<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn open(reader: R) -> AssetResult<Self> {
        let asset = SectionedAssetView::open(reader).await?;
        if asset.header().version != MESH0_VERSION_0 {
            return Err(AssetError::UnsupportedFormatVersion(asset.header().version));
        }
        asset.table().require(section_kind::INFO, 0)?;
        Ok(Self { asset })
    }

    pub fn asset(&self) -> &SectionedAssetView<R> {
        &self.asset
    }

    pub fn header(&self) -> &SectionedAssetHeader {
        self.asset.header()
    }

    pub fn section_table(&self) -> &SectionTable {
        self.asset.table()
    }

    pub async fn info(&self) -> AssetResult<Mesh0InfoSectionOwned> {
        decode_info_section(
            self.asset
                .section(section_kind::INFO, 0)?
                .read_all()
                .await?,
        )
    }

    pub async fn material_slots(&self) -> AssetResult<Option<Mesh0MaterialSlotsSectionOwned>> {
        let Some(record) = self.asset.table().get(section_kind::MATERIAL_SLOTS, 0) else {
            return Ok(None);
        };
        Ok(Some(decode_material_slots_section(
            self.asset.section_from_record(*record).read_all().await?,
        )?))
    }

    pub async fn skinning(&self) -> AssetResult<Option<Mesh0SkinningSectionOwned>> {
        let Some(record) = self.asset.table().get(section_kind::SKINNING, 0) else {
            return Ok(None);
        };
        Ok(Some(decode_skinning_section(
            self.asset.section_from_record(*record).read_all().await?,
        )?))
    }

    pub async fn lod(&self, lod_level: u32) -> AssetResult<Mesh0LodSectionView<R>> {
        Mesh0LodSectionView::open(self.asset.section(section_kind::LOD, lod_level)?).await
    }

    pub fn lod_records(&self) -> impl Iterator<Item = &SectionRecord> {
        self.asset.table().find_all(section_kind::LOD)
    }

    pub async fn read_owned(&self) -> AssetResult<Mesh0Owned> {
        let mut sections = Vec::with_capacity(self.asset.table().len());
        for record in self.asset.table().records() {
            let view = self.asset.section_from_record(*record);
            let body = match record.kind {
                section_kind::INFO => {
                    Mesh0SectionBodyOwned::Info(decode_info_section(view.read_all().await?)?)
                }
                section_kind::MATERIAL_SLOTS => Mesh0SectionBodyOwned::MaterialSlots(
                    decode_material_slots_section(view.read_all().await?)?,
                ),
                section_kind::SKINNING => Mesh0SectionBodyOwned::Skinning(decode_skinning_section(
                    view.read_all().await?,
                )?),
                section_kind::SKELETON_REFS => {
                    Mesh0SectionBodyOwned::SkeletonRefs(decode_asset_refs(view.read_all().await?)?)
                }
                section_kind::ANIMATION_REFS => {
                    Mesh0SectionBodyOwned::AnimationRefs(decode_asset_refs(view.read_all().await?)?)
                }
                section_kind::EFFECT_REFS => {
                    Mesh0SectionBodyOwned::EffectRefs(decode_asset_refs(view.read_all().await?)?)
                }
                section_kind::COLLISION_REFS => {
                    Mesh0SectionBodyOwned::CollisionRefs(decode_asset_refs(view.read_all().await?)?)
                }
                section_kind::ATTACHMENT_REFS => Mesh0SectionBodyOwned::AttachmentRefs(
                    decode_asset_refs(view.read_all().await?)?,
                ),
                section_kind::SOURCE_FEATURES => Mesh0SectionBodyOwned::SourceFeatures(
                    decode_source_features_section(view.read_all().await?)?,
                ),
                section_kind::SOURCE_DEBUG => {
                    Mesh0SectionBodyOwned::SourceDebug(view.read_all().await?)
                }
                section_kind::LOD => {
                    let lod = Mesh0LodSectionView::open(view).await?.read_owned().await?;
                    Mesh0SectionBodyOwned::Lod(Box::new(lod))
                }
                _ => Mesh0SectionBodyOwned::Raw(view.read_all().await?),
            };
            sections.push(Mesh0SectionOwned {
                kind: record.kind,
                key: record.key,
                flags: record.flags,
                extra: record.extra,
                body,
            });
        }
        Ok(Mesh0Owned {
            version: self.asset.header().version,
            sections,
        })
    }
}

pub struct Mesh0LodSectionView<R> {
    view: SectionView<R>,
    header: Mesh0LodHeader,
    submeshes: OnceCell<Vec<Mesh0Submesh>>,
    draw_batches: OnceCell<Vec<Mesh0DrawBatch>>,
    joint_palette: OnceCell<Vec<Mesh0JointPaletteEntry>>,
}

impl<R> Mesh0LodSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub async fn open(view: SectionView<R>) -> AssetResult<Self> {
        let header_bytes = view.read_local(0, Mesh0LodHeader::BYTE_SIZE as u64).await?;
        let header = decode_lod_header(header_bytes)?;
        Ok(Self {
            view,
            header,
            submeshes: OnceCell::new(),
            draw_batches: OnceCell::new(),
            joint_palette: OnceCell::new(),
        })
    }

    pub fn header(&self) -> &Mesh0LodHeader {
        &self.header
    }

    pub async fn submeshes(&self) -> AssetResult<&Vec<Mesh0Submesh>> {
        self.submeshes
            .get_or_try_init(|| async {
                read_table_from_span(&self.view, self.header.submeshes, Mesh0Submesh::BYTE_SIZE)
                    .await
            })
            .await
    }

    pub async fn draw_batches(&self) -> AssetResult<&Vec<Mesh0DrawBatch>> {
        self.draw_batches
            .get_or_try_init(|| async {
                read_table_from_span(
                    &self.view,
                    self.header.draw_batches,
                    Mesh0DrawBatch::BYTE_SIZE,
                )
                .await
            })
            .await
    }

    pub async fn joint_palette(&self) -> AssetResult<&Vec<Mesh0JointPaletteEntry>> {
        self.joint_palette
            .get_or_try_init(|| async {
                read_table_from_span(
                    &self.view,
                    self.header.joint_palette,
                    Mesh0JointPaletteEntry::BYTE_SIZE,
                )
                .await
            })
            .await
    }

    pub async fn vertex_bytes(&self) -> AssetResult<Bytes> {
        self.view
            .read_local(
                u64::from(self.header.vertex_buffer.offset),
                u64::from(self.header.vertex_buffer.size),
            )
            .await
    }

    pub async fn index_bytes(&self) -> AssetResult<Bytes> {
        self.view
            .read_local(
                u64::from(self.header.index_buffer.offset),
                u64::from(self.header.index_buffer.size),
            )
            .await
    }

    pub async fn read_owned(&self) -> AssetResult<Mesh0LodSectionOwned> {
        Ok(Mesh0LodSectionOwned {
            header: self.header.clone(),
            submeshes: self.submeshes().await?.clone(),
            draw_batches: self.draw_batches().await?.clone(),
            joint_palette: self.joint_palette().await?.clone(),
            vertex_bytes: self.vertex_bytes().await?,
            index_bytes: self.index_bytes().await?,
        })
    }
}

async fn read_table_from_span<R, T>(
    view: &SectionView<R>,
    span: TableSpan,
    expected_stride: usize,
) -> AssetResult<Vec<T>>
where
    R: AssetRead + Clone + Send + Sync,
    T: DecodeTableRow,
{
    if span.stride != expected_stride as u32 && span.count != 0 {
        return Err(AssetError::InvalidData("invalid table stride"));
    }
    let size = u64::from(span.count)
        .checked_mul(u64::from(span.stride))
        .ok_or(AssetError::OffsetOverflow)?;
    let bytes = view.read_local(u64::from(span.offset), size).await?;
    decode_table(&bytes, expected_stride, T::decode_row)
}

trait DecodeTableRow: Sized {
    fn decode_row(cursor: &mut file_core::DecodeCursor<'_>) -> AssetResult<Self>;
}

impl DecodeTableRow for Mesh0Submesh {
    fn decode_row(cursor: &mut file_core::DecodeCursor<'_>) -> AssetResult<Self> {
        // Reuse full-section decoder by decoding a temporary one-row table.
        let start = cursor.position();
        let row = cursor.read_bytes(Self::BYTE_SIZE)?;
        let mut rows = decode_table(row, Self::BYTE_SIZE, |c| {
            Ok(Mesh0Submesh {
                submesh_id: c.read_u32_le()?,
                flags: c.read_u32_le()?,
                vertex_start: c.read_u32_le()?,
                vertex_count: c.read_u32_le()?,
                index_start: c.read_u32_le()?,
                index_count: c.read_u32_le()?,
                material_slot: c.read_u32_le()?,
                joint_palette_start: c.read_u32_le()?,
                joint_palette_count: c.read_u32_le()?,
                max_bone_influence: c.read_u32_le()?,
                center: [c.read_f32_le()?, c.read_f32_le()?, c.read_f32_le()?],
                sort_center: [c.read_f32_le()?, c.read_f32_le()?, c.read_f32_le()?],
                bounding_radius: c.read_f32_le()?,
                source_submesh_id: c.read_u32_le()?,
                source_level: c.read_u32_le()?,
            })
        })?;
        cursor.seek(start + Self::BYTE_SIZE)?;
        Ok(rows.remove(0))
    }
}

impl DecodeTableRow for Mesh0DrawBatch {
    fn decode_row(cursor: &mut file_core::DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Mesh0DrawBatch {
            batch_id: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
            submesh_index: cursor.read_u32_le()?,
            material_slot: cursor.read_u32_le()?,
            render_queue: cursor.read_u32_le()?,
            shader_hint: cursor.read_u32_le()?,
            priority: cursor.read_i32_le()?,
            vertex_start: cursor.read_u32_le()?,
            vertex_count: cursor.read_u32_le()?,
            index_start: cursor.read_u32_le()?,
            index_count: cursor.read_u32_le()?,
            source_skin_batch_index: cursor.read_u32_le()?,
            source_skin_section_index: cursor.read_u32_le()?,
            source_geoset_index: cursor.read_u32_le()?,
            source_material_index: cursor.read_u32_le()?,
            source_texture_combo_index: cursor.read_u32_le()?,
        })
    }
}

impl DecodeTableRow for Mesh0JointPaletteEntry {
    fn decode_row(cursor: &mut file_core::DecodeCursor<'_>) -> AssetResult<Self> {
        Ok(Mesh0JointPaletteEntry {
            local_joint_index: cursor.read_u32_le()?,
            skeleton_joint_index: cursor.read_u32_le()?,
            source_bone_index: cursor.read_u32_le()?,
            flags: cursor.read_u32_le()?,
        })
    }
}
