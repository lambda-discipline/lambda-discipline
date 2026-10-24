//! Conversions from `usize` counts to the `u32` ids, slots and offsets the
//! compiler stores to keep its IR small.

/// `n` as a `u32`, for ids and slots numbered by counting.
///
/// # Panics
///
/// If `n` does not fit, that is, a program has more than 2^32 locals, type
/// variables or similar. Source files are limited to [`MAX_FILE_LEN`] bytes
/// when they are loaded, so byte offsets always fit.
///
/// [`MAX_FILE_LEN`]: crate::source::MAX_FILE_LEN
pub(crate) fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("count exceeds u32::MAX")
}

/// A length or count as an lbd `Int`. Rust lengths never exceed
/// `isize::MAX`, so this cannot fail.
pub(crate) fn to_i64(n: usize) -> i64 {
    i64::try_from(n).expect("lengths fit in i64")
}
