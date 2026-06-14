pub mod mesh0_builder;
pub mod mesh0_reader;
pub mod sections;

pub use mesh0_builder::*;
pub use mesh0_reader::*;
pub use sections::*;

pub const MESH0_VERSION_0: u32 = 0;

pub mod section_type {
    pub const MESH_INFO: u32 = 1;
    pub const MATERIAL_SLOTS: u32 = 2;
    pub const SKINNING: u32 = 3;
    pub const SKELETON_REFS: u32 = 4;
    pub const ANIMATION_REFS: u32 = 5;
    pub const EFFECT_REFS: u32 = 6;
    pub const COLLISION_REFS: u32 = 7;
    pub const ATTACHMENT_REFS: u32 = 8;
    pub const SOURCE_FEATURES: u32 = 9;
    pub const SOURCE_DEBUG: u32 = 10;
    pub const RENDER_VARIANT: u32 = 11;
}
