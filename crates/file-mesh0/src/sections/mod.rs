use file_core::AssetId128;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Mesh0Info {
    pub mesh_flags: u32,
    pub lod_count: u32,
    pub default_lod: u32,
    pub material_slot_count: u32,
    pub skinning_section_count: u32,
    pub source_feature_count: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounding_sphere_center: [f32; 3],
    pub bounding_sphere_radius: f32,
    pub source_format: u32,
    pub source_version: u32,
}

#[derive(Debug, Clone)]
pub struct Mesh0InfoSectionOwned {
    pub info: Mesh0Info,
}

#[derive(Debug, Clone)]
pub struct Mesh0MaterialSlot {
    pub slot_index: u32,
    pub flags: u32,
    pub material_asset: AssetId128,
    pub render_queue: u32,
    pub shader_hint: u32,
    pub source_material_index: u32,
    pub source_texture_combo_index: u32,
    pub source_texture_count: u32,
    pub name_hash: u64,
}

#[derive(Debug, Clone)]
pub struct Mesh0MaterialSlotsSectionOwned {
    pub slots: Vec<Mesh0MaterialSlot>,
}

#[derive(Debug, Clone)]
pub struct Mesh0SkinningSectionOwned {
    pub skeleton_asset: AssetId128,
    pub flags: u32,
    pub joint_count_hint: u32,
    pub max_weights_per_vertex: u32,
    pub joint_index_format: u32,
    pub weight_format: u32,
    pub source_bone_count: u32,
    pub source_key_bone_count: u32,
}

#[derive(Debug, Clone)]
pub struct Mesh0AssetRef {
    pub asset: AssetId128,
    pub flags: u32,
    pub kind: u32,
    pub source_index: u32,
}

#[derive(Debug, Clone)]
pub struct Mesh0SourceFeature {
    pub feature_kind: u32,
    pub support_status: u32,
    pub mapped_target_kind: u32,
    pub mapped_target_index: u32,
    pub source_index: u32,
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct Mesh0SourceFeaturesSectionOwned {
    pub features: Vec<Mesh0SourceFeature>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSpan {
    pub offset: u32,
    pub count: u32,
    pub stride: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalBlobSpan {
    pub offset: u32,
    pub size: u32,
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
    pub vertex_buffer: LocalBlobSpan,
    pub index_buffer: LocalBlobSpan,
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

#[derive(Debug, Clone)]
pub struct Mesh0JointPaletteEntry {
    pub local_joint_index: u32,
    pub skeleton_joint_index: u32,
    pub source_bone_index: u32,
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct Mesh0LodSectionOwned {
    pub header: Mesh0LodHeader,
    pub submeshes: Vec<Mesh0Submesh>,
    pub draw_batches: Vec<Mesh0DrawBatch>,
    pub joint_palette: Vec<Mesh0JointPaletteEntry>,
    pub vertex_bytes: Bytes,
    pub index_bytes: Bytes,
}
