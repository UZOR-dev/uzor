//! Noto Sans CJK SC Regular (OFL-1.1, Adobe), half b. The font (16,437,364 bytes) is too big
//! for one crates.io crate, so it ships as two halves; `uzor-fonts`' build script appends
//! `uzor-fonts-cjk-b` to `uzor-fonts-cjk-a` and checks the length.

/// Bytes of half b of `NotoSansCJKsc-Regular.otf` (Version 2.004).
pub const PART: &[u8] = include_bytes!("../fonts/NotoSansCJKsc-Regular.otf.part-b");
