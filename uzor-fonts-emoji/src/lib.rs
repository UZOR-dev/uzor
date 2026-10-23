//! Noto Color Emoji (OFL-1.1, Google), shipped as a crate so `uzor-fonts` builds without
//! downloading anything. `uzor-fonts`' build script is the only intended user.

/// `NotoColorEmoji.ttf` (Version 2.051, 10,673,480 bytes).
pub const NOTO_COLOR_EMOJI: &[u8] = include_bytes!("../fonts/NotoColorEmoji.ttf");
