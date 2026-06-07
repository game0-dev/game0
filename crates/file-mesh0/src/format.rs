pub const MESH0_VERSION_0: u32 = 0;

pub mod index_format {
    pub const UINT16: u32 = 1;
    pub const UINT32: u32 = 2;
}

pub mod primitive_topology {
    pub const TRIANGLE_LIST: u32 = 1;
}

pub mod render_queue {
    pub const OPAQUE: u32 = 1;
    pub const ALPHA_TEST: u32 = 2;
    pub const TRANSPARENT: u32 = 3;
    pub const ADDITIVE: u32 = 4;
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

pub mod support_status {
    pub const MAPPED: u32 = 1;
    pub const IGNORED_INTENTIONAL: u32 = 2;
    pub const PRESERVED_DEBUG_ONLY: u32 = 3;
    pub const UNSUPPORTED: u32 = 4;
}
