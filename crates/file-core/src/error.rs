pub type AssetResult<T> = Result<T, AssetError>;

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("unexpected eof")]
    UnexpectedEof,

    #[error("unsupported format version: {0}")]
    UnsupportedFormatVersion(u32),

    #[error("invalid section kind: {0}")]
    InvalidSectionKind(u32),

    #[error("missing required section: kind={0}, key={1}")]
    MissingRequiredSection(u32, u32),

    #[error("duplicate section: kind={kind}, key={key}")]
    DuplicateSection { kind: u32, key: u32 },

    #[error("offset overflow")]
    OffsetOverflow,

    #[error("range out of bounds")]
    RangeOutOfBounds,

    #[error("invalid data: {0}")]
    InvalidData(&'static str),

    #[error("io error: {0}")]
    Io(String),
}

impl From<std::num::TryFromIntError> for AssetError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::OffsetOverflow
    }
}
