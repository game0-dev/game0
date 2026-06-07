use std::ops::Range;

use crate::{AssetError, AssetResult};

pub fn checked_range(total_len: u64, offset: u64, size: u64) -> AssetResult<Range<usize>> {
    let end = offset.checked_add(size).ok_or(AssetError::OffsetOverflow)?;
    if end > total_len {
        return Err(AssetError::RangeOutOfBounds);
    }
    Ok(usize::try_from(offset)?..usize::try_from(end)?)
}

pub fn align_up(value: usize, align: usize) -> AssetResult<usize> {
    if align <= 1 {
        return Ok(value);
    }
    let add = align.checked_sub(1).ok_or(AssetError::OffsetOverflow)?;
    let padded = value.checked_add(add).ok_or(AssetError::OffsetOverflow)?;
    Ok(padded / align * align)
}

pub fn is_aligned(value: u64, align: u64) -> bool {
    align <= 1 || value % align == 0
}
