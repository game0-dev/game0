use bytes::Bytes;
use file_core::{
    align::{align_up, checked_range},
    decode_table, read_f32x3, AssetError, AssetRead, AssetResult, DecodeCursor, EncodeBuffer,
    OffsetAssetReader,
};
use tokio::sync::OnceCell;

pub mod index_format {
    pub const UINT16: u32 = 1;
    pub const UINT32: u32 = 2;
}

pub mod primitive_topology {
    pub const TRIANGLE_LIST: u32 = 1;
}

pub mod vertex_layout {
    pub const POSITION_NORMAL_UV0: u32 = 1;
    pub const POSITION_NORMAL_UV0_SKINNED: u32 = 2;
}

pub mod vertex_attribute {
    pub const POSITION: u32 = 1 << 0;
    pub const NORMAL: u32 = 1 << 1;
    pub const TANGENT: u32 = 1 << 2;
    pub const UV0: u32 = 1 << 3;
    pub const UV1: u32 = 1 << 4;
    pub const COLOR0: u32 = 1 << 5;
    pub const JOINTS0: u32 = 1 << 6;
    pub const WEIGHTS0: u32 = 1 << 7;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSpan {
    pub offset: u32,
    pub count: u32,
    pub stride: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobSpan {
    pub offset: u32,
    pub size: u32,
}

#[derive(Clone)]
pub struct BlobView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    section_len: u32,
    span: BlobSpan,
}

impl<R> BlobView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub(crate) fn new(reader: OffsetAssetReader<R>, section_len: u32, span: BlobSpan) -> Self {
        Self {
            reader,
            section_len,
            span,
        }
    }

    pub fn span(&self) -> BlobSpan {
        self.span
    }

    pub async fn read(&self) -> AssetResult<Bytes> {
        checked_range(
            u64::from(self.section_len),
            u64::from(self.span.offset),
            u64::from(self.span.size),
        )?;
        self.reader
            .read_at(u64::from(self.span.offset), u64::from(self.span.size))
            .await
    }
}

struct SectionBodyWriter {
    body: Vec<u8>,
}

impl SectionBodyWriter {
    fn with_reserved_header(header_size: usize) -> Self {
        Self {
            body: vec![0; header_size],
        }
    }

    fn append_blob(&mut self, bytes: &Bytes, align: usize) -> AssetResult<BlobSpan> {
        let offset = align_up(self.body.len(), align)?;
        self.body.resize(offset, 0);
        self.body.extend_from_slice(bytes);
        Ok(BlobSpan {
            offset: u32::try_from(offset)?,
            size: u32::try_from(bytes.len())?,
        })
    }

    fn into_inner(self) -> Vec<u8> {
        self.body
    }
}

#[derive(Debug, Clone)]
pub struct Mesh0LodHeader {
    pub lod_level: u32,
    pub lod_flags: u32,
    pub screen_size: f32,
    pub max_distance: f32,
    pub primitive_topology: u32,
    pub vertex_layout_id: u32,
    pub vertex_attribute_mask: u32,
    pub vertex_stride: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub index_format: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounding_sphere_center: [f32; 3],
    pub bounding_sphere_radius: f32,
    pub submeshes: TableSpan,
    pub draw_batches: TableSpan,
    pub joint_palette: TableSpan,
    pub vertex_buffer: BlobSpan,
    pub index_buffer: BlobSpan,
}

impl Mesh0LodHeader {
    pub const BYTE_SIZE: usize = 136;
}

#[derive(Debug, Clone)]
pub struct Mesh0Submesh {
    pub submesh_id: u32,
    pub flags: u32,
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
    pub material_slot: u32,
    pub joint_palette_start: u32,
    pub joint_palette_count: u32,
    pub max_bone_influence: u32,
    pub center: [f32; 3],
    pub sort_center: [f32; 3],
    pub bounding_radius: f32,
    pub source_submesh_id: u32,
    pub source_level: u32,
}

impl Mesh0Submesh {
    pub const BYTE_SIZE: usize = 76;
}

#[derive(Debug, Clone)]
pub struct Mesh0DrawBatch {
    pub batch_id: u32,
    pub flags: u32,
    pub submesh_index: u32,
    pub material_slot: u32,
    pub render_queue: u32,
    pub shader_hint: u32,
    pub priority: i32,
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
    pub source_skin_batch_index: u32,
    pub source_skin_section_index: u32,
    pub source_geoset_index: u32,
    pub source_material_index: u32,
    pub source_texture_combo_index: u32,
}

impl Mesh0DrawBatch {
    pub const BYTE_SIZE: usize = 64;
}

#[derive(Debug, Clone)]
pub struct Mesh0JointPaletteEntry {
    pub local_joint_index: u32,
    pub skeleton_joint_index: u32,
    pub source_bone_index: u32,
    pub flags: u32,
}

impl Mesh0JointPaletteEntry {
    pub const BYTE_SIZE: usize = 16;
}

#[derive(Debug, Clone)]
pub struct LodSectionOwned {
    pub header: Mesh0LodHeader,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
    pub vertex_bytes: Bytes,
    pub index_bytes: Bytes,
}

impl LodSectionOwned {
    pub fn write(&self) -> AssetResult<Bytes> {
        encode_lod_section(self)
    }
}

#[derive(Clone)]
pub struct LodSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    reader: OffsetAssetReader<R>,
    len: u32,
    header: OnceCell<Mesh0LodHeader>,
    submeshes: OnceCell<Vec<Mesh0Submesh>>,
    draw_batches: OnceCell<Vec<Mesh0DrawBatch>>,
    joint_palette: OnceCell<Vec<Mesh0JointPaletteEntry>>,
}

impl<R> LodSectionView<R>
where
    R: AssetRead + Clone + Send + Sync,
{
    pub(crate) fn new(reader: OffsetAssetReader<R>, len: u32) -> Self {
        Self {
            reader,
            len,
            header: OnceCell::new(),
            submeshes: OnceCell::new(),
            draw_batches: OnceCell::new(),
            joint_palette: OnceCell::new(),
        }
    }

    pub async fn header(&self) -> AssetResult<&Mesh0LodHeader> {
        self.header
            .get_or_try_init(|| async {
                if self.len < Mesh0LodHeader::BYTE_SIZE as u32 {
                    return Err(AssetError::UnexpectedEof);
                }
                let header_bytes = self
                    .reader
                    .read_at(0, Mesh0LodHeader::BYTE_SIZE as u64)
                    .await?;
                decode_lod_header(header_bytes)
            })
            .await
    }

    pub async fn submeshes(&self) -> AssetResult<&Vec<Mesh0Submesh>> {
        self.submeshes
            .get_or_try_init(|| async {
                let span = self.header().await?.submeshes;
                read_table_from_span(
                    &self.reader,
                    self.len,
                    span,
                    Mesh0Submesh::BYTE_SIZE,
                    decode_submesh_table,
                )
                .await
            })
            .await
    }

    pub async fn draw_batches(&self) -> AssetResult<&Vec<Mesh0DrawBatch>> {
        self.draw_batches
            .get_or_try_init(|| async {
                let span = self.header().await?.draw_batches;
                read_table_from_span(
                    &self.reader,
                    self.len,
                    span,
                    Mesh0DrawBatch::BYTE_SIZE,
                    decode_draw_batch_table,
                )
                .await
            })
            .await
    }

    pub async fn joint_palette(&self) -> AssetResult<&Vec<Mesh0JointPaletteEntry>> {
        self.joint_palette
            .get_or_try_init(|| async {
                let span = self.header().await?.joint_palette;
                read_table_from_span(
                    &self.reader,
                    self.len,
                    span,
                    Mesh0JointPaletteEntry::BYTE_SIZE,
                    decode_joint_palette_table,
                )
                .await
            })
            .await
    }

    pub async fn vertex_bytes(&self) -> AssetResult<Bytes> {
        self.vertex_buffer().await?.read().await
    }

    pub async fn index_bytes(&self) -> AssetResult<Bytes> {
        self.index_buffer().await?.read().await
    }

    pub async fn vertex_buffer(&self) -> AssetResult<BlobView<R>> {
        Ok(BlobView::new(
            self.reader.clone(),
            self.len,
            self.header().await?.vertex_buffer,
        ))
    }

    pub async fn index_buffer(&self) -> AssetResult<BlobView<R>> {
        Ok(BlobView::new(
            self.reader.clone(),
            self.len,
            self.header().await?.index_buffer,
        ))
    }

    pub async fn read_owned(&self) -> AssetResult<LodSectionOwned> {
        Ok(LodSectionOwned {
            header: self.header().await?.clone(),
            submeshes: self.submeshes().await?.clone(),
            draw_batches: self.draw_batches().await?.clone(),
            joint_palette: self.joint_palette().await?.clone(),
            vertex_bytes: self.vertex_bytes().await?,
            index_bytes: self.index_bytes().await?,
        })
    }
}

pub fn decode_lod_header(bytes: Bytes) -> AssetResult<Mesh0LodHeader> {
    if bytes.len() != Mesh0LodHeader::BYTE_SIZE {
        return Err(AssetError::UnexpectedEof);
    }
    let mut cursor = DecodeCursor::new(&bytes);
    Ok(Mesh0LodHeader {
        lod_level: cursor.read_u32_le()?,
        lod_flags: cursor.read_u32_le()?,
        screen_size: cursor.read_f32_le()?,
        max_distance: cursor.read_f32_le()?,
        primitive_topology: cursor.read_u32_le()?,
        vertex_layout_id: cursor.read_u32_le()?,
        vertex_attribute_mask: cursor.read_u32_le()?,
        vertex_stride: cursor.read_u32_le()?,
        vertex_count: cursor.read_u32_le()?,
        index_count: cursor.read_u32_le()?,
        index_format: cursor.read_u32_le()?,
        bounds_min: read_f32x3(&mut cursor)?,
        bounds_max: read_f32x3(&mut cursor)?,
        bounding_sphere_center: read_f32x3(&mut cursor)?,
        bounding_sphere_radius: cursor.read_f32_le()?,
        submeshes: read_table_span(&mut cursor)?,
        draw_batches: read_table_span(&mut cursor)?,
        joint_palette: read_table_span(&mut cursor)?,
        vertex_buffer: read_blob_span(&mut cursor)?,
        index_buffer: read_blob_span(&mut cursor)?,
    })
}

pub fn decode_lod_section(bytes: Bytes) -> AssetResult<LodSectionOwned> {
    let header = decode_lod_header(bytes.slice(0..Mesh0LodHeader::BYTE_SIZE))?;
    let submeshes = read_table_span_owned(
        &bytes,
        header.submeshes,
        Mesh0Submesh::BYTE_SIZE,
        decode_submesh,
    )?;
    let draw_batches = read_table_span_owned(
        &bytes,
        header.draw_batches,
        Mesh0DrawBatch::BYTE_SIZE,
        decode_draw_batch,
    )?;
    let joint_palette = read_table_span_owned(
        &bytes,
        header.joint_palette,
        Mesh0JointPaletteEntry::BYTE_SIZE,
        decode_joint_palette_entry,
    )?;
    let vertex_range = checked_range(
        bytes.len() as u64,
        u64::from(header.vertex_buffer.offset),
        u64::from(header.vertex_buffer.size),
    )?;
    let index_range = checked_range(
        bytes.len() as u64,
        u64::from(header.index_buffer.offset),
        u64::from(header.index_buffer.size),
    )?;
    Ok(LodSectionOwned {
        header,
        submeshes,
        draw_batches,
        joint_palette,
        vertex_bytes: bytes.slice(vertex_range),
        index_bytes: bytes.slice(index_range),
    })
}

pub fn decode_submesh_table(bytes: &[u8]) -> AssetResult<Vec<Mesh0Submesh>> {
    decode_table(bytes, Mesh0Submesh::BYTE_SIZE, decode_submesh)
}

pub fn decode_draw_batch_table(bytes: &[u8]) -> AssetResult<Vec<Mesh0DrawBatch>> {
    decode_table(bytes, Mesh0DrawBatch::BYTE_SIZE, decode_draw_batch)
}

pub fn decode_joint_palette_table(bytes: &[u8]) -> AssetResult<Vec<Mesh0JointPaletteEntry>> {
    decode_table(
        bytes,
        Mesh0JointPaletteEntry::BYTE_SIZE,
        decode_joint_palette_entry,
    )
}

pub fn encode_lod_section(section: &LodSectionOwned) -> AssetResult<Bytes> {
    let mut writer = SectionBodyWriter::with_reserved_header(Mesh0LodHeader::BYTE_SIZE);
    let submeshes = append_table(&mut writer.body, &section.submeshes, encode_submesh)?;
    let draw_batches = append_table(&mut writer.body, &section.draw_batches, encode_draw_batch)?;
    let joint_palette = append_table(
        &mut writer.body,
        &section.joint_palette,
        encode_joint_palette,
    )?;
    let vertex_buffer = writer.append_blob(&section.vertex_bytes, 8)?;
    let index_buffer = writer.append_blob(&section.index_bytes, 8)?;

    let mut header = section.header.clone();
    header.submeshes = submeshes;
    header.draw_batches = draw_batches;
    header.joint_palette = joint_palette;
    header.vertex_buffer = vertex_buffer;
    header.index_buffer = index_buffer;

    let mut header_bytes = EncodeBuffer::new();
    encode_lod_header(&header, &mut header_bytes);
    let mut body = writer.into_inner();
    body[..Mesh0LodHeader::BYTE_SIZE].copy_from_slice(&header_bytes.into_inner());
    Ok(Bytes::from(body))
}

pub(crate) fn validate_lod_section(
    lod: &LodSectionOwned,
    material_slot_count: u32,
) -> AssetResult<()> {
    let header = &lod.header;
    if header.vertex_stride == 0 {
        return Err(AssetError::InvalidData(
            "vertex_stride must be greater than zero",
        ));
    }
    if header.vertex_count == 0 || header.index_count == 0 {
        return Err(AssetError::InvalidData("empty LOD buffers are invalid"));
    }
    if header.primitive_topology != primitive_topology::TRIANGLE_LIST {
        return Err(AssetError::InvalidData("unsupported primitive topology"));
    }
    let index_size = match header.index_format {
        index_format::UINT16 => 2,
        index_format::UINT32 => 4,
        _ => return Err(AssetError::InvalidData("invalid index format")),
    };
    let expected_vertices = header
        .vertex_count
        .checked_mul(header.vertex_stride)
        .ok_or(AssetError::OffsetOverflow)?;
    let expected_indices = header
        .index_count
        .checked_mul(index_size)
        .ok_or(AssetError::OffsetOverflow)?;
    if lod.vertex_bytes.len() != expected_vertices as usize {
        return Err(AssetError::InvalidData("invalid vertex buffer size"));
    }
    if lod.index_bytes.len() != expected_indices as usize {
        return Err(AssetError::InvalidData("invalid index buffer size"));
    }
    if !valid_bounds(header.bounds_min, header.bounds_max) {
        return Err(AssetError::InvalidData("invalid LOD bounds"));
    }

    for submesh in &lod.submeshes {
        checked_range_u32(
            submesh.vertex_start,
            submesh.vertex_count,
            header.vertex_count,
        )?;
        checked_range_u32(submesh.index_start, submesh.index_count, header.index_count)?;
        checked_range_u32(
            submesh.joint_palette_start,
            submesh.joint_palette_count,
            lod.joint_palette.len() as u32,
        )?;
        if submesh.material_slot >= material_slot_count {
            return Err(AssetError::InvalidData(
                "submesh material slot out of range",
            ));
        }
    }
    for batch in &lod.draw_batches {
        checked_range_u32(batch.vertex_start, batch.vertex_count, header.vertex_count)?;
        checked_range_u32(batch.index_start, batch.index_count, header.index_count)?;
        if batch.submesh_index >= lod.submeshes.len() as u32 {
            return Err(AssetError::InvalidData("draw batch submesh out of range"));
        }
        if batch.material_slot >= material_slot_count {
            return Err(AssetError::InvalidData(
                "draw batch material slot out of range",
            ));
        }
    }
    Ok(())
}

fn checked_range_u32(start: u32, count: u32, len: u32) -> AssetResult<()> {
    if start.checked_add(count).filter(|end| *end <= len).is_none() {
        return Err(AssetError::RangeOutOfBounds);
    }
    Ok(())
}

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}

async fn read_table_from_span<R, T>(
    reader: &OffsetAssetReader<R>,
    section_len: u32,
    span: TableSpan,
    expected_stride: usize,
    decode: fn(&[u8]) -> AssetResult<Vec<T>>,
) -> AssetResult<Vec<T>>
where
    R: AssetRead + Clone + Send + Sync,
{
    if span.stride != expected_stride as u32 && span.count != 0 {
        return Err(AssetError::InvalidData("invalid table stride"));
    }
    let size = u64::from(span.count)
        .checked_mul(u64::from(span.stride))
        .ok_or(AssetError::OffsetOverflow)?;
    checked_range(u64::from(section_len), u64::from(span.offset), size)?;
    let bytes = reader.read_at(u64::from(span.offset), size).await?;
    decode(&bytes)
}

fn read_table_span_owned<T>(
    bytes: &Bytes,
    span: TableSpan,
    expected_stride: usize,
    decode: fn(&mut DecodeCursor<'_>) -> AssetResult<T>,
) -> AssetResult<Vec<T>> {
    if span.stride != expected_stride as u32 {
        return Err(AssetError::InvalidData("invalid table stride"));
    }
    let size = u64::from(span.count)
        .checked_mul(u64::from(span.stride))
        .ok_or(AssetError::OffsetOverflow)?;
    let range = checked_range(bytes.len() as u64, u64::from(span.offset), size)?;
    decode_table(&bytes[range], expected_stride, decode)
}

fn append_table<T>(
    body: &mut Vec<u8>,
    values: &[T],
    encode: fn(&T, &mut EncodeBuffer),
) -> AssetResult<TableSpan> {
    let offset = align_up(body.len(), 8)?;
    body.resize(offset, 0);
    let mut table = EncodeBuffer::new();
    for value in values {
        encode(value, &mut table);
    }
    let bytes = table.into_inner();
    let stride = if values.is_empty() {
        0
    } else {
        bytes.len() / values.len()
    };
    body.extend_from_slice(&bytes);
    Ok(TableSpan {
        offset: u32::try_from(offset)?,
        count: u32::try_from(values.len())?,
        stride: u32::try_from(stride)?,
    })
}

fn encode_lod_header(header: &Mesh0LodHeader, out: &mut EncodeBuffer) {
    out.write_u32_le(header.lod_level);
    out.write_u32_le(header.lod_flags);
    out.write_f32_le(header.screen_size);
    out.write_f32_le(header.max_distance);
    out.write_u32_le(header.primitive_topology);
    out.write_u32_le(header.vertex_layout_id);
    out.write_u32_le(header.vertex_attribute_mask);
    out.write_u32_le(header.vertex_stride);
    out.write_u32_le(header.vertex_count);
    out.write_u32_le(header.index_count);
    out.write_u32_le(header.index_format);
    out.write_f32x3(header.bounds_min);
    out.write_f32x3(header.bounds_max);
    out.write_f32x3(header.bounding_sphere_center);
    out.write_f32_le(header.bounding_sphere_radius);
    write_table_span(out, header.submeshes);
    write_table_span(out, header.draw_batches);
    write_table_span(out, header.joint_palette);
    write_blob_span(out, header.vertex_buffer);
    write_blob_span(out, header.index_buffer);
}

fn encode_submesh(value: &Mesh0Submesh, out: &mut EncodeBuffer) {
    out.write_u32_le(value.submesh_id);
    out.write_u32_le(value.flags);
    out.write_u32_le(value.vertex_start);
    out.write_u32_le(value.vertex_count);
    out.write_u32_le(value.index_start);
    out.write_u32_le(value.index_count);
    out.write_u32_le(value.material_slot);
    out.write_u32_le(value.joint_palette_start);
    out.write_u32_le(value.joint_palette_count);
    out.write_u32_le(value.max_bone_influence);
    out.write_f32x3(value.center);
    out.write_f32x3(value.sort_center);
    out.write_f32_le(value.bounding_radius);
    out.write_u32_le(value.source_submesh_id);
    out.write_u32_le(value.source_level);
}

fn encode_draw_batch(value: &Mesh0DrawBatch, out: &mut EncodeBuffer) {
    out.write_u32_le(value.batch_id);
    out.write_u32_le(value.flags);
    out.write_u32_le(value.submesh_index);
    out.write_u32_le(value.material_slot);
    out.write_u32_le(value.render_queue);
    out.write_u32_le(value.shader_hint);
    out.write_i32_le(value.priority);
    out.write_u32_le(value.vertex_start);
    out.write_u32_le(value.vertex_count);
    out.write_u32_le(value.index_start);
    out.write_u32_le(value.index_count);
    out.write_u32_le(value.source_skin_batch_index);
    out.write_u32_le(value.source_skin_section_index);
    out.write_u32_le(value.source_geoset_index);
    out.write_u32_le(value.source_material_index);
    out.write_u32_le(value.source_texture_combo_index);
}

fn encode_joint_palette(value: &Mesh0JointPaletteEntry, out: &mut EncodeBuffer) {
    out.write_u32_le(value.local_joint_index);
    out.write_u32_le(value.skeleton_joint_index);
    out.write_u32_le(value.source_bone_index);
    out.write_u32_le(value.flags);
}

fn decode_submesh(cursor: &mut DecodeCursor<'_>) -> AssetResult<Mesh0Submesh> {
    Ok(Mesh0Submesh {
        submesh_id: cursor.read_u32_le()?,
        flags: cursor.read_u32_le()?,
        vertex_start: cursor.read_u32_le()?,
        vertex_count: cursor.read_u32_le()?,
        index_start: cursor.read_u32_le()?,
        index_count: cursor.read_u32_le()?,
        material_slot: cursor.read_u32_le()?,
        joint_palette_start: cursor.read_u32_le()?,
        joint_palette_count: cursor.read_u32_le()?,
        max_bone_influence: cursor.read_u32_le()?,
        center: read_f32x3(cursor)?,
        sort_center: read_f32x3(cursor)?,
        bounding_radius: cursor.read_f32_le()?,
        source_submesh_id: cursor.read_u32_le()?,
        source_level: cursor.read_u32_le()?,
    })
}

fn decode_draw_batch(cursor: &mut DecodeCursor<'_>) -> AssetResult<Mesh0DrawBatch> {
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

fn decode_joint_palette_entry(
    cursor: &mut DecodeCursor<'_>,
) -> AssetResult<Mesh0JointPaletteEntry> {
    Ok(Mesh0JointPaletteEntry {
        local_joint_index: cursor.read_u32_le()?,
        skeleton_joint_index: cursor.read_u32_le()?,
        source_bone_index: cursor.read_u32_le()?,
        flags: cursor.read_u32_le()?,
    })
}

fn read_table_span(cursor: &mut DecodeCursor<'_>) -> AssetResult<TableSpan> {
    Ok(TableSpan {
        offset: cursor.read_u32_le()?,
        count: cursor.read_u32_le()?,
        stride: cursor.read_u32_le()?,
    })
}

fn read_blob_span(cursor: &mut DecodeCursor<'_>) -> AssetResult<BlobSpan> {
    Ok(BlobSpan {
        offset: cursor.read_u32_le()?,
        size: cursor.read_u32_le()?,
    })
}

fn write_table_span(out: &mut EncodeBuffer, span: TableSpan) {
    out.write_u32_le(span.offset);
    out.write_u32_le(span.count);
    out.write_u32_le(span.stride);
}

fn write_blob_span(out: &mut EncodeBuffer, span: BlobSpan) {
    out.write_u32_le(span.offset);
    out.write_u32_le(span.size);
}
