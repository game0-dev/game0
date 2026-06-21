use std::sync::{Arc, Mutex};

use bytes::Bytes;
use file_anim0::{
    Anim0AnimationClip, Anim0BoneAnimationTrack, Anim0QuatKey, Anim0TrackInfo, Anim0Vec3Key,
};
use file_core::{align::is_aligned, AssetReader, AssetResult, MemoryAssetReader};

use file_mesh0::*;

const HEADER_BYTE_SIZE: u64 = 8;
const SECTION_ENTRY_BYTE_SIZE: u64 = 16;
const RENDER_VARIANT_HEADER_BYTE_SIZE: u64 = 20;

#[derive(Clone)]
struct RecordingReader {
    inner: MemoryAssetReader,
    base_offset: u64,
    reads: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl RecordingReader {
    fn new(bytes: Bytes) -> Self {
        Self {
            inner: MemoryAssetReader::new(bytes),
            base_offset: 0,
            reads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn reads(&self) -> Vec<(u64, u64)> {
        self.reads.lock().unwrap().clone()
    }
}

impl AssetReader for RecordingReader {
    fn with_offset_accumulate(&self, offset: u64) -> AssetResult<Self> {
        let mut reader = self.clone();
        reader.base_offset = reader
            .base_offset
            .checked_add(offset)
            .ok_or(file_core::AssetError::OffsetOverflow)?;
        Ok(reader)
    }

    fn with_file(&self, _file_id: u32) -> AssetResult<Self> {
        Err(file_core::AssetError::InvalidData(
            "recording reader does not support external files",
        ))
    }

    async fn read_at(&self, offset: u64, len: u64) -> AssetResult<Bytes> {
        let absolute = self
            .base_offset
            .checked_add(offset)
            .ok_or(file_core::AssetError::OffsetOverflow)?;
        self.reads.lock().unwrap().push((absolute, len));
        self.inner.read_at(absolute, len).await
    }
}

#[test]
fn open_reads_only_header_and_table() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let _ = Mesh0Reader::open(reader.clone()).await.unwrap();
        assert_eq!(
            reader.reads(),
            vec![(0, HEADER_BYTE_SIZE), (8, 2 * SECTION_ENTRY_BYTE_SIZE)]
        );
    });
}

#[test]
fn builder_write_layout_is_aligned() {
    let bytes = sample_mesh().write_bytes().unwrap();
    let first_offset = u64::from(read_u32_le(&bytes[16..20]));
    let second_offset = u64::from(read_u32_le(&bytes[32..36]));
    assert!(is_aligned(first_offset, 8));
    assert!(is_aligned(second_offset, 8));
}

#[test]
fn mesh_info_header_is_lazy() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let mesh = Mesh0Reader::open(reader.clone()).await.unwrap();

        let before_info = reader.reads().len();
        let info = mesh.read_mesh_info().await.unwrap();
        assert_eq!(info.bounding_box_min, [0.0; 3]);
        assert_eq!(info.bounding_box_max, [1.0; 3]);
        assert_eq!(info.bounding_sphere_radius, 1.0);
        assert_eq!(reader.reads().len(), before_info + 1);
        assert_eq!(reader.reads()[before_info].1, MeshInfoHeader::BYTE_SIZE);
    });
}

#[test]
fn render_variant_metadata_is_lazy_and_indices_are_lazy() {
    pollster::block_on(async {
        let reader = RecordingReader::new(sample_mesh().write_bytes().unwrap());
        let mesh = Mesh0Reader::open(reader.clone()).await.unwrap();

        let before_variant = reader.reads().len();
        let variant = mesh.read_render_variant(0).await.unwrap();
        assert_eq!(variant.submeshes.len(), 1);
        assert_eq!(reader.reads().len(), before_variant + 2);
        assert_eq!(
            reader.reads()[before_variant].1,
            RENDER_VARIANT_HEADER_BYTE_SIZE
        );
        assert_eq!(
            reader.reads()[before_variant + 1].1,
            RENDER_VARIANT_HEADER_BYTE_SIZE + Mesh0Submesh::BYTE_SIZE + Mesh0DrawBatch::BYTE_SIZE
        );

        let before_vertices = reader.reads().len();
        let vertices = variant.vertex_bytes().await.unwrap();
        assert_eq!(
            vertices.len(),
            RenderVariantReader::<RecordingReader>::VERTEX_SIZE * 3
        );
        assert_eq!(reader.reads().len(), before_vertices + 1);
        assert_eq!(
            reader.reads().last().unwrap().1,
            u64::try_from(RenderVariantReader::<RecordingReader>::VERTEX_SIZE * 3).unwrap()
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
        let _ = mesh.read_mesh_info().await.unwrap();
        let after_first = reader.reads().len();
        let _ = mesh.read_mesh_info().await.unwrap();

        assert!(after_first > before);
        assert_eq!(reader.reads().len(), after_first);
    });
}

#[test]
fn duplicate_singleton_section_is_rejected() {
    pollster::block_on(async {
        let mut bytes = sample_mesh().write_bytes().unwrap().to_vec();
        let mesh_info_section_type = bytes[8..12].to_vec();
        bytes[24..28].copy_from_slice(&mesh_info_section_type);
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
fn animation_section_roundtrip_preserves_sequence_identity() {
    let section = AnimationReader {
        clips: vec![
            Anim0AnimationClip {
                sequence_index: 7,
                animation_id: 16,
                sub_animation_id: 0,
                duration_ms: 900,
                flags: 1,
                movement_speed: 0.0,
                frequency: 0,
                replay_range_ms: None,
                bounds: None,
                next_animation: None,
                aliasing: None,
                bone_tracks: vec![Anim0BoneAnimationTrack {
                    bone_index: 3,
                    translation_info: Anim0TrackInfo::default(),
                    translations: vec![Anim0Vec3Key {
                        time_ms: 0,
                        value: [1.0, 2.0, 3.0],
                    }],
                    rotation_info: Anim0TrackInfo::default(),
                    rotations: vec![Anim0QuatKey {
                        time_ms: 0,
                        value: [0.0, 0.0, 0.0, 1.0],
                    }],
                    scale_info: Anim0TrackInfo::default(),
                    scales: vec![Anim0Vec3Key {
                        time_ms: 0,
                        value: [1.0, 1.0, 1.0],
                    }],
                }],
            },
            Anim0AnimationClip {
                sequence_index: 8,
                animation_id: 16,
                sub_animation_id: 1,
                duration_ms: 700,
                flags: 0,
                movement_speed: 0.0,
                frequency: 0,
                replay_range_ms: None,
                bounds: None,
                next_animation: None,
                aliasing: None,
                bone_tracks: Vec::new(),
            },
        ],
    };

    let decoded = AnimationReader::read_bytes(section.write().unwrap()).unwrap();
    assert_eq!(decoded.clips.len(), 2);
    assert_eq!(decoded.clips[0].sequence_index, 7);
    assert_eq!(decoded.clips[0].animation_id, 16);
    assert_eq!(decoded.clips[0].bone_tracks[0].bone_index, 3);
    assert_eq!(decoded.clips[1].sequence_index, 8);
    assert_eq!(decoded.clips[1].sub_animation_id, 1);
}

fn sample_mesh() -> Mesh0Builder {
    let info = MeshInfoHeader {
        bounding_box_min: [0.0; 3],
        bounding_box_max: [1.0; 3],
        bounding_sphere_radius: 1.0,
    };
    let variant = RenderVariantBuilder {
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
            skin_batch_index: 0,
            skin_section_index: 0,
            geoset_index: 0,
            texture_combo_index: 0,
        }],
        joint_palette: Vec::new(),
        vertex_bytes: Bytes::from(vec![
            0;
            RenderVariantReader::<MemoryAssetReader>::VERTEX_SIZE
                * 3
        ]),
        index_bytes: Bytes::from(vec![0, 0, 1, 0, 2, 0]),
    };

    let mut mesh = Mesh0Builder::new(info);
    mesh.push_render_variant(variant);
    mesh
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}
