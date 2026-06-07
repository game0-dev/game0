#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AssetId128(pub [u8; 16]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AssetKind(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AssetRef {
    pub kind: AssetKind,
    pub id: AssetId128,
}

pub mod asset_kind {
    pub const MESH0: u32 = 1;
    pub const TEX0: u32 = 2;
    pub const MAT0: u32 = 3;
    pub const SKEL0: u32 = 4;
    pub const ANIM0: u32 = 5;
    pub const EFFECT0: u32 = 6;
    pub const COLL0: u32 = 7;
}
