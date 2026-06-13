use std::sync::{Arc, Mutex};

use bytes::Bytes;
use file_core::{align::is_aligned, AssetRead, AssetResult, MemoryAssetReader};

use file_mesh0::{sections::*, Mesh0Owned, Mesh0View};

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
fn sectioned_asset_open_reads_only_header_and_table() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write().unwrap());
        let _ = Mesh0View::open(reader.clone()).await.unwrap();
        assert_eq!(
            reader.reads(),
            vec![(0, HEADER_BYTE_SIZE), (8, 3 * SECTION_ENTRY_BYTE_SIZE)]
        );
    });
}

#[test]
fn mesh0_open_accepts_duplicate_section_type() {
    pollster::block_on(async {
        let mut bytes = sample_mesh().write().unwrap().to_vec();
        bytes[20..24].copy_from_slice(&section_type::MESH_INFO.to_le_bytes());
        assert!(Mesh0View::open(MemoryAssetReader::new(Bytes::from(bytes)))
            .await
            .is_ok());
    });
}

#[test]
fn sectioned_asset_encode_layout_is_aligned() {
    let bytes = sample_mesh().write().unwrap();
    let first_offset = u64::from(read_u32_le(&bytes[12..16]));
    let second_offset = u64::from(read_u32_le(&bytes[24..28]));
    assert!(is_aligned(first_offset, 8));
    assert!(is_aligned(second_offset, 8));
}

#[test]
fn mesh0_owned_encode_then_open() {
    pollster::block_on(async {
        let mesh = sample_mesh();
        let bytes = mesh.write().unwrap();
        let view = Mesh0View::open(MemoryAssetReader::new(bytes))
            .await
            .unwrap();
        let item = view
            .sections_by_type(section_type::MESH_INFO)
            .next()
            .unwrap();
        let info = item.read_section_view().await.unwrap();
        let SectionView::MeshInfo(info) = info else {
            panic!("expected MESH_INFO section");
        };
        assert_eq!(info.default_lod, 0);
    });
}

#[test]
fn mesh0_open_does_not_read_lod_body() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write().unwrap());
        let _ = Mesh0View::open(reader.clone()).await.unwrap();
        assert_eq!(
            reader.reads(),
            vec![(0, HEADER_BYTE_SIZE), (8, 3 * SECTION_ENTRY_BYTE_SIZE)]
        );
    });
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

#[test]
fn mesh0_lod_view_reads_metadata_without_blobs() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write().unwrap());
        let view = Mesh0View::open(reader.clone()).await.unwrap();
        let item = view.sections_by_type(section_type::LOD).next().unwrap();

        let before = reader.reads().len();
        let lod = item.read_section_view().await.unwrap();
        let SectionView::Lod(lod) = lod else {
            panic!("expected LOD section");
        };
        assert_eq!(lod.header.vertex_count, 3);
        let reads = reader.reads();
        assert_eq!(reads.len(), before + 2);
        assert_eq!(reads[before].1, Mesh0LodHeader::BYTE_SIZE as u64);
        assert_eq!(
            reads[before + 1].1,
            (Mesh0LodHeader::BYTE_SIZE + Mesh0Submesh::BYTE_SIZE + Mesh0DrawBatch::BYTE_SIZE)
                as u64
        );
    });
}

#[test]
fn section_table_item_caches_section_view() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write().unwrap());
        let view = Mesh0View::open(reader.clone()).await.unwrap();
        let item = view.sections_by_type(section_type::LOD).next().unwrap();

        let before = reader.reads().len();
        let _ = item.read_section_view().await.unwrap();
        let after_first = reader.reads().len();
        let _ = item.read_section_view().await.unwrap();

        assert!(after_first > before);
        assert_eq!(reader.reads().len(), after_first);
    });
}

#[test]
fn mesh0_lod_vertex_bytes_are_lazy() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write().unwrap());
        let view = Mesh0View::open(reader.clone()).await.unwrap();
        let item = view.sections_by_type(section_type::LOD).next().unwrap();
        let lod = item.read_section_view().await.unwrap();
        let SectionView::Lod(lod) = lod else {
            panic!("expected LOD section");
        };
        let before = reader.reads().len();
        let vertices = lod.vertex_bytes().await.unwrap();
        assert_eq!(vertices.len(), 96);
        let reads = reader.reads();
        assert_eq!(reads.len(), before + 1);
        assert_eq!(reads.last().unwrap().1, 96);
    });
}

#[test]
fn mesh0_read_owned_roundtrip() {
    pollster::block_on(async {
        let mesh = sample_mesh();
        let bytes = mesh.write().unwrap();
        let view = Mesh0View::open(MemoryAssetReader::new(bytes))
            .await
            .unwrap();
        let owned = view.read_owned().await.unwrap();
        let bytes2 = owned.write().unwrap();
        let view2 = Mesh0View::open(MemoryAssetReader::new(bytes2))
            .await
            .unwrap();
        let item = view2
            .sections_by_type(section_type::MESH_INFO)
            .next()
            .unwrap();
        let info = item.read_section_view().await.unwrap();
        let SectionView::MeshInfo(info) = info else {
            panic!("expected MESH_INFO section");
        };
        assert_eq!(info.default_lod, 0);
    });
}

#[test]
fn mesh0_invalid_lod_range_rejected() {
    let mut mesh = sample_mesh();
    let lod = mesh
        .sections
        .iter_mut()
        .find_map(|section| match section {
            SectionOwned::Lod(lod) => Some(lod.as_mut()),
            _ => None,
        })
        .unwrap();
    lod.submeshes[0].index_count = 999;
    assert!(mesh.write().is_err());
}

fn sample_mesh() -> Mesh0Owned {
    let info = MeshInfoSection {
        mesh_flags: 0,
        default_lod: 0,
        bounds_min: [0.0; 3],
        bounds_max: [1.0; 3],
        bounding_sphere_center: [0.5; 3],
        bounding_sphere_radius: 1.0,
        source_format: 1,
        source_version: 0,
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
    let lod = LodSectionOwned {
        header: Mesh0LodHeader {
            lod_level: 0,
            lod_flags: 0,
            screen_size: 1.0,
            max_distance: 100.0,
            primitive_topology: primitive_topology::TRIANGLE_LIST,
            vertex_layout_id: vertex_layout::POSITION_NORMAL_UV0,
            vertex_attribute_mask: vertex_attribute::POSITION
                | vertex_attribute::NORMAL
                | vertex_attribute::UV0,
            vertex_stride: 32,
            vertex_count: 3,
            index_count: 3,
            index_format: index_format::UINT16,
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            bounding_sphere_center: [0.5; 3],
            bounding_sphere_radius: 1.0,
            submesh_count: 0,
            draw_batch_count: 0,
            joint_palette_count: 0,
            vertex_buffer_size: 0,
            index_buffer_size: 0,
        },
        submeshes: vec![Mesh0Submesh {
            submesh_id: 0,
            flags: 0,
            vertex_start: 0,
            vertex_count: 3,
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
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
            source_skin_batch_index: 0,
            source_skin_section_index: 0,
            source_geoset_index: 0,
            source_material_index: 0,
            source_texture_combo_index: 0,
        }],
        joint_palette: Vec::new(),
        vertex_bytes: Bytes::from(vec![0; 96]),
        index_bytes: Bytes::from(vec![0; 6]),
    };
    Mesh0Owned {
        sections: vec![
            SectionOwned::MeshInfo(info),
            SectionOwned::MaterialSlots(materials),
            SectionOwned::Lod(Box::new(lod)),
        ],
    }
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}
