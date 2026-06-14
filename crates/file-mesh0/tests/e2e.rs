use std::sync::{Arc, Mutex};

use bytes::Bytes;
use file_core::{align::is_aligned, AssetRead, AssetResult, MemoryAssetReader};

use file_mesh0::*;

const HEADER_BYTE_SIZE: u64 = 8;
const SECTION_ENTRY_BYTE_SIZE: u64 = 12;

#[derive(Clone)]
struct RecordingReader {
    inner: MemoryAssetReader,
    reads: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl RecordingReader {
    fn new(bytes: Bytes) -> Self {
        Self {
            inner: MemoryAssetReader::new(bytes),
            reads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn reads(&self) -> Vec<(u64, u64)> {
        self.reads.lock().unwrap().clone()
    }
}

impl AssetRead for RecordingReader {
    async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
        self.reads.lock().unwrap().push((offset, len));
        self.inner.read_at(offset, len).await
    }
}

#[test]
fn open_reads_only_header_and_table() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let _ = Mesh0Reader::open(reader.clone()).await.unwrap();
        assert_eq!(
            reader.reads(),
            vec![(0, HEADER_BYTE_SIZE), (8, 3 * SECTION_ENTRY_BYTE_SIZE)]
        );
    });
}

#[test]
fn builder_write_layout_is_aligned() {
    let bytes = sample_mesh().write_bytes().unwrap();
    let first_offset = u64::from(read_u32_le(&bytes[12..16]));
    let second_offset = u64::from(read_u32_le(&bytes[24..28]));
    let third_offset = u64::from(read_u32_le(&bytes[36..40]));
    assert!(is_aligned(first_offset, 8));
    assert!(is_aligned(second_offset, 8));
    assert!(is_aligned(third_offset, 8));
}

#[test]
fn mesh_info_header_is_lazy_and_vertices_are_lazy() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let mesh = Mesh0Reader::open(reader.clone()).await.unwrap();

        let before_info = reader.reads().len();
        let info = mesh.mesh_info().await.unwrap();
        assert_eq!(info.header.default_lod, 0);
        assert_eq!(info.header.vertex_count, 3);
        assert_eq!(reader.reads().len(), before_info + 1);
        assert_eq!(
            reader.reads()[before_info].1,
            MeshInfoHeader::BYTE_SIZE as u64
        );

        let before_vertices = reader.reads().len();
        let vertices = info.vertex_bytes().await.unwrap();
        assert_eq!(vertices.len(), 96);
        assert_eq!(reader.reads().len(), before_vertices + 1);
        assert_eq!(reader.reads().last().unwrap().1, 96);
    });
}

#[test]
fn render_variant_metadata_is_lazy_and_indices_are_lazy() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let mesh = Mesh0Reader::open(reader.clone()).await.unwrap();

        let before_variant = reader.reads().len();
        let variant = mesh.render_variant(0).await.unwrap();
        assert_eq!(variant.header.index_count, 3);
        assert_eq!(variant.submeshes.len(), 1);
        assert_eq!(reader.reads().len(), before_variant + 2);
        assert_eq!(
            reader.reads()[before_variant].1,
            RenderVariantHeader::BYTE_SIZE as u64
        );
        assert_eq!(
            reader.reads()[before_variant + 1].1,
            (RenderVariantHeader::BYTE_SIZE + Mesh0Submesh::BYTE_SIZE + Mesh0DrawBatch::BYTE_SIZE)
                as u64
        );

        let before_indices = reader.reads().len();
        let indices = variant.index_bytes().await.unwrap();
        assert_eq!(indices.len(), 6);
        assert_eq!(reader.reads().len(), before_indices + 1);
        assert_eq!(reader.reads().last().unwrap().1, 6);
    });
}

#[test]
fn section_readers_are_cached() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let mesh = Mesh0Reader::open(reader.clone()).await.unwrap();

        let before = reader.reads().len();
        let _ = mesh.mesh_info().await.unwrap();
        let after_first = reader.reads().len();
        let _ = mesh.mesh_info().await.unwrap();

        assert!(after_first > before);
        assert_eq!(reader.reads().len(), after_first);
    });
}

#[test]
fn duplicate_singleton_section_is_rejected() {
    pollster::block_on(async {
        let mut bytes = sample_mesh().write_bytes().unwrap().to_vec();
        bytes[32..36].copy_from_slice(&section_type::MESH_INFO.to_le_bytes());
        assert!(
            Mesh0Reader::open(MemoryAssetReader::new(Bytes::from(bytes)))
                .await
                .is_err()
        );
    });
}

#[test]
fn invalid_index_reference_is_rejected() {
    let mut mesh = sample_mesh();
    mesh.render_variants[0].index_bytes = Bytes::from(vec![9, 0, 1, 0, 2, 0]);
    assert!(mesh.write_bytes().is_err());
}

#[test]
fn ref_section_u64_roundtrip() {
    let refs = EffectRefsSection {
        refs: vec![10, 20, 30],
    };
    let bytes = refs.write().unwrap();
    assert_eq!(bytes.len(), 24);
    let decoded = EffectRefsSection::read(bytes).unwrap();
    assert_eq!(decoded.refs, refs.refs);
}

fn sample_mesh() -> Mesh0Builder {
    let info = MeshInfoBuilder {
        header: MeshInfoHeader {
            mesh_flags: 0,
            default_lod: 0,
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            bounding_sphere_center: [0.5; 3],
            bounding_sphere_radius: 1.0,
            source_format: 1,
            source_version: 0,
            primitive_topology: primitive_topology::TRIANGLE_LIST,
            vertex_layout_id: vertex_layout::POSITION_NORMAL_UV0,
            vertex_attribute_mask: vertex_attribute::POSITION
                | vertex_attribute::NORMAL
                | vertex_attribute::UV0,
            vertex_stride: 32,
            vertex_count: 3,
            vertex_buffer_size: 0,
        },
        vertex_bytes: Bytes::from(vec![0; 96]),
    };
    let materials = MaterialSlotsSection {
        slots: vec![Mesh0MaterialSlot {
            slot_index: 0,
            flags: 0,
            material_asset: 1,
            render_queue: render_queue::OPAQUE,
            shader_hint: 0,
            source_material_index: 0,
            source_texture_combo_index: 0,
            source_texture_count: 1,
            name_hash: 1,
        }],
    };
    let variant = RenderVariantBuilder {
        header: RenderVariantHeader {
            render_variant_index: 0,
            render_variant_flags: 0,
            lod_level: NO_LOD_LEVEL,
            screen_size: 0.0,
            max_distance: 0.0,
            primitive_topology: primitive_topology::TRIANGLE_LIST,
            index_count: 3,
            index_format: index_format::UINT16,
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            bounding_sphere_center: [0.5; 3],
            bounding_sphere_radius: 1.0,
            submesh_count: 0,
            draw_batch_count: 0,
            joint_palette_count: 0,
            index_buffer_size: 0,
        },
        submeshes: vec![Mesh0Submesh {
            submesh_id: 0,
            flags: 0,
            index_start: 0,
            index_count: 3,
            material_slot: 0,
            joint_palette_start: 0,
            joint_palette_count: 0,
            max_bone_influence: 0,
            center: [0.5; 3],
            sort_center: [0.5; 3],
            bounding_radius: 1.0,
            source_submesh_id: 0,
            source_level: 0,
        }],
        draw_batches: vec![Mesh0DrawBatch {
            batch_id: 0,
            flags: 0,
            submesh_index: 0,
            material_slot: 0,
            render_queue: render_queue::OPAQUE,
            shader_hint: 0,
            priority: 0,
            index_start: 0,
            index_count: 3,
            source_skin_batch_index: 0,
            source_skin_section_index: 0,
            source_geoset_index: 0,
            source_material_index: 0,
            source_texture_combo_index: 0,
        }],
        joint_palette: Vec::new(),
        index_bytes: Bytes::from(vec![0, 0, 1, 0, 2, 0]),
    };

    let mut mesh = Mesh0Builder::new(info);
    mesh.material_slots = Some(materials);
    mesh.push_render_variant(variant);
    mesh
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}
