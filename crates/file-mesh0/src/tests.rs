use std::sync::{Arc, Mutex};

use file_core::{AssetId128, AssetRead, AssetResult, MemoryAssetReader};
use bytes::Bytes;

use crate::{
    format::{
        index_format, primitive_topology, render_queue, vertex_attribute, vertex_layout,
        MESH0_VERSION_0,
    },
    section::{Mesh0SectionBodyOwned, Mesh0SectionOwned},
    section_kind,
    sections::*,
    view::Mesh0View,
    Mesh0Owned,
};

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
fn mesh0_owned_encode_then_open() {
    pollster::block_on(async {
        let mesh = sample_mesh();
        let bytes = mesh.encode().unwrap();
        let view = Mesh0View::open(MemoryAssetReader::new(bytes))
            .await
            .unwrap();
        assert_eq!(view.header().version, MESH0_VERSION_0);
        assert_eq!(view.section_table().len(), 3);
        assert_eq!(view.info().await.unwrap().info.lod_count, 1);
    });
}

#[test]
fn mesh0_open_does_not_read_lod_body() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().encode().unwrap());
        let _ = Mesh0View::open(reader.clone()).await.unwrap();
        assert_eq!(reader.reads(), vec![(0, 8), (8, 3 * 32)]);
    });
}

#[test]
fn mesh0_lod_open_reads_only_lod_header() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().encode().unwrap());
        let view = Mesh0View::open(reader.clone()).await.unwrap();
        let before = reader.reads().len();
        let _lod = view.lod(0).await.unwrap();
        let reads = reader.reads();
        assert_eq!(reads.len(), before + 1);
        assert_eq!(reads.last().unwrap().1, Mesh0LodHeader::BYTE_SIZE as u64);
    });
}

#[test]
fn mesh0_lod_vertex_bytes_are_lazy() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().encode().unwrap());
        let view = Mesh0View::open(reader.clone()).await.unwrap();
        let lod = view.lod(0).await.unwrap();
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
        let bytes = mesh.encode().unwrap();
        let view = Mesh0View::open(MemoryAssetReader::new(bytes))
            .await
            .unwrap();
        let owned = view.read_owned().await.unwrap();
        owned.validate().unwrap();
        let bytes2 = owned.encode().unwrap();
        let view2 = Mesh0View::open(MemoryAssetReader::new(bytes2))
            .await
            .unwrap();
        assert_eq!(view2.info().await.unwrap().info.lod_count, 1);
    });
}

#[test]
fn mesh0_invalid_lod_range_rejected() {
    let mut mesh = sample_mesh();
    let lod = mesh
        .sections
        .iter_mut()
        .find_map(|section| match &mut section.body {
            Mesh0SectionBodyOwned::Lod(lod) => Some(lod.as_mut()),
            _ => None,
        })
        .unwrap();
    lod.submeshes[0].index_count = 999;
    assert!(mesh.validate().is_err());
}

fn sample_mesh() -> Mesh0Owned {
    let info = Mesh0InfoSectionOwned {
        info: Mesh0Info {
            mesh_flags: 0,
            lod_count: 1,
            default_lod: 0,
            material_slot_count: 1,
            skinning_section_count: 0,
            source_feature_count: 0,
            bounds_min: [0.0; 3],
            bounds_max: [1.0; 3],
            bounding_sphere_center: [0.5; 3],
            bounding_sphere_radius: 1.0,
            source_format: 1,
            source_version: 0,
        },
    };
    let materials = Mesh0MaterialSlotsSectionOwned {
        slots: vec![Mesh0MaterialSlot {
            slot_index: 0,
            flags: 0,
            material_asset: AssetId128([1; 16]),
            render_queue: render_queue::OPAQUE,
            shader_hint: 0,
            source_material_index: 0,
            source_texture_combo_index: 0,
            source_texture_count: 1,
            name_hash: 1,
        }],
    };
    let lod = Mesh0LodSectionOwned {
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
            submeshes: TableSpan {
                offset: 0,
                count: 0,
                stride: 0,
            },
            draw_batches: TableSpan {
                offset: 0,
                count: 0,
                stride: 0,
            },
            joint_palette: TableSpan {
                offset: 0,
                count: 0,
                stride: 0,
            },
            vertex_buffer: LocalBlobSpan { offset: 0, size: 0 },
            index_buffer: LocalBlobSpan { offset: 0, size: 0 },
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
        version: MESH0_VERSION_0,
        sections: vec![
            Mesh0SectionOwned {
                kind: section_kind::INFO,
                key: 0,
                flags: 0,
                extra: 0,
                body: Mesh0SectionBodyOwned::Info(info),
            },
            Mesh0SectionOwned {
                kind: section_kind::MATERIAL_SLOTS,
                key: 0,
                flags: 0,
                extra: 0,
                body: Mesh0SectionBodyOwned::MaterialSlots(materials),
            },
            Mesh0SectionOwned {
                kind: section_kind::LOD,
                key: 0,
                flags: 0,
                extra: 0,
                body: Mesh0SectionBodyOwned::Lod(Box::new(lod)),
            },
        ],
    }
}
