pub mod mesh_info;
pub mod render_variant;

pub(crate) const MESH_INFO: u32 = 1;
pub(crate) const RENDER_VARIANT: u32 = 2;
pub(crate) const SKELETON: u32 = 3;
pub(crate) const ANIMATION: u32 = 4;

pub use file_anim0::Anim0Reader as AnimationReader;
pub use file_skeleton0::Skeleton0Reader as SkeletonReader;
pub use mesh_info::MeshInfoHeader;
pub use render_variant::{
    render_queue, Mesh0DrawBatch, Mesh0JointPaletteEntry, Mesh0Submesh, RenderVariantBuilder,
    RenderVariantReader,
};
