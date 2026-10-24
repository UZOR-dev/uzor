//! Critter colour tokens (the app.js avatar palette).

/// Body colours, picked by name hash.
pub const PALETTE: [&str; 10] = [
    "#1677ff", "#ff7a1a", "#ff3d8b", "#8b5cf6", "#14b8a6", "#a855f7", "#22c55e", "#f5b014",
    "#ef4444", "#06b6d4",
];
/// Eye fill.
pub const EYE: &str = "#0b0b0d";
/// Thinking dots.
pub const THOUGHT: &str = "#9ec5ff";
/// Attention ring.
pub const ATTENTION: &str = "#e0a63a";

/// Shape indices of direct-message critters (kept here: a four-number table reads
/// like an RGBA literal to the colour lint).
pub const DM_SHAPES: &[usize] = &[0, 5, 9, 11];
