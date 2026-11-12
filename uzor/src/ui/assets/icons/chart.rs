//! Chart and trading-tool icons — bar types, drawing tools, harmonic / Elliott /
//! SMC / order-flow patterns, scanners, tool families and order-flow panel slots.
//! From the mylittlechart icon collection (the ones [`super::ui`] did not have yet).
//! viewBox 0 0 24 24, `stroke="currentColor"`, thin (1.25) pen; plain consts, so an
//! app pays only for the icons it names.

/// Range bar (mylittlechart).
pub const ICON_RANGE_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 3v4"/>
  <path d="M6 17v4"/>
  <rect x="4" y="7" width="4" height="10" rx="1"/>
  <path d="M12 3v4"/>
  <path d="M12 17v4"/>
  <rect x="10" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M18 3v4"/>
  <path d="M18 17v4"/>
  <rect x="16" y="7" width="4" height="10" rx="1"/>
</svg>"##;

/// Volume bar (mylittlechart).
pub const ICON_VOLUME_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 3v4"/>
  <path d="M6 17v4"/>
  <rect x="4" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M12 3v4"/>
  <path d="M12 17v4"/>
  <rect x="10" y="7" width="4" height="10" rx="1"/>
  <path d="M18 3v4"/>
  <path d="M18 17v4"/>
  <rect x="16" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M2 21h20" stroke-width="1.125" opacity="0.5"/>
</svg>"##;

/// Dollar bar (mylittlechart).
pub const ICON_DOLLAR_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M5 3v4"/>
  <path d="M5 17v4"/>
  <rect x="3" y="7" width="4" height="10" rx="1"/>
  <path d="M14 17v4"/>
  <rect x="12" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M20.5 6.3c-.4-.5-1.1-.8-1.8-.8-1.2 0-2.2.8-2.2 1.8s1 1.6 2.2 1.9c1.2.3 2.2.9 2.2 1.9s-1 1.8-2.2 1.8c-.7 0-1.4-.3-1.8-.8"/>
  <path d="M18.7 4.5v9"/>
</svg>"##;

/// Tick bar (mylittlechart).
pub const ICON_TICK_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 3v4"/>
  <path d="M6 17v4"/>
  <rect x="4" y="7" width="4" height="10" rx="1"/>
  <path d="M12 3v4"/>
  <path d="M12 17v4"/>
  <rect x="10" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M18 3v4"/>
  <path d="M18 17v4"/>
  <rect x="16" y="7" width="4" height="10" rx="1"/>
  <path d="M2 1.5v2" stroke-width="1.125"/>
  <path d="M8 1.5v2" stroke-width="1.125"/>
  <path d="M14 1.5v2" stroke-width="1.125"/>
  <path d="M20 1.5v2" stroke-width="1.125"/>
</svg>"##;

/// Tick imbalance bar (mylittlechart).
pub const ICON_TICK_IMBALANCE_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 3v4"/>
  <path d="M6 17v4"/>
  <rect x="4" y="7" width="4" height="10" rx="1"/>
  <path d="M12 3v4"/>
  <path d="M12 17v4"/>
  <rect x="10" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M18 3v4"/>
  <path d="M18 17v4"/>
  <rect x="16" y="7" width="4" height="10" rx="1"/>
  <path d="M2 21h20" stroke-width="1.125" stroke-dasharray="1 2.4" opacity="0.7"/>
</svg>"##;

/// Volume imbalance bar (mylittlechart).
pub const ICON_VOLUME_IMBALANCE_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 3v4"/>
  <path d="M6 17v4"/>
  <rect x="4" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M12 3v4"/>
  <path d="M12 17v4"/>
  <rect x="10" y="7" width="4" height="10" rx="1"/>
  <path d="M18 3v4"/>
  <path d="M18 17v4"/>
  <rect x="16" y="7" width="4" height="10" rx="1" fill="currentColor"/>
  <path d="M2 21h20" stroke-width="1.125" stroke-dasharray="1 2.4" opacity="0.7"/>
</svg>"##;

/// Run bar (mylittlechart).
pub const ICON_RUN_BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 20V15"/>
  <path d="M9 20V11"/>
  <path d="M14 20V7"/>
  <path d="M19 20V4"/>
  <path d="M2 20h20" stroke-width="1.125" opacity="0.6"/>
</svg>"##;

/// Renko (mylittlechart).
pub const ICON_RENKO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="2" y="16" width="5" height="5" fill="currentColor"/>
  <rect x="7" y="11" width="5" height="5" fill="currentColor"/>
  <rect x="12" y="6" width="5" height="5"/>
  <rect x="17" y="1" width="5" height="5"/>
</svg>"##;

/// Pnf (mylittlechart).
pub const ICON_PNF: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 4l5 5M8 4l-5 5"/>
  <path d="M3 11l5 5M8 11l-5 5"/>
  <circle cx="17.5" cy="8.5" r="2.5"/>
  <circle cx="17.5" cy="15.5" r="2.5"/>
</svg>"##;

/// Kagi (mylittlechart).
pub const ICON_KAGI: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.25">
  <path d="M4 6h6" stroke-width="1.125"/>
  <path d="M10 6v9" stroke-width="1.125"/>
  <path d="M10 15h6" stroke-width="2.625"/>
  <path d="M16 15V4" stroke-width="2.625"/>
  <path d="M16 4h4" stroke-width="1.125"/>
  <path d="M20 4v16" stroke-width="1.125"/>
</svg>"##;

/// Three line break (mylittlechart).
pub const ICON_THREE_LINE_BREAK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="2" y="15" width="5" height="6" fill="currentColor"/>
  <rect x="8" y="10" width="5" height="6" fill="currentColor"/>
  <rect x="14" y="4" width="5" height="7" fill="currentColor"/>
</svg>"##;

/// Footprint cells (mylittlechart).
pub const ICON_FOOTPRINT_CELLS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="3" y="3" width="18" height="18" rx="1" stroke-width="1.5"/>
  <path d="M3 9h18"/>
  <path d="M3 15h18"/>
  <path d="M9 3v18"/>
  <path d="M15 3v18"/>
  <rect x="9" y="9" width="6" height="6" fill="currentColor" fill-opacity="0.35" stroke="none"/>
</svg>"##;

/// Tpo (mylittlechart).
pub const ICON_TPO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 5h4"/>
  <path d="M3 9h9"/>
  <path d="M3 13h14"/>
  <path d="M3 17h9"/>
  <path d="M3 21h4"/>
</svg>"##;

/// Ray left (mylittlechart).
pub const ICON_RAY_LEFT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M20 16L4 8"/>
  <circle cx="20" cy="16" r="2" fill="currentColor"/>
  <path d="M6 6L4 8l2 2"/>
</svg>"##;

/// Horizontal ray left (mylittlechart).
pub const ICON_HORIZONTAL_RAY_LEFT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M20 12H3"/>
  <circle cx="20" cy="12" r="2" fill="currentColor"/>
  <path d="M5 9L2 12l3 3"/>
</svg>"##;

/// Schiff Pitchfork — median origin shifted to the midpoint of the first
/// two tines (marked dot), not the handle center.
pub const ICON_SCHIFF_PITCHFORK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 2v6c0 2 2 3 6 3s6-1 6-3V2"/>
  <path d="M6 2v4"/>
  <path d="M12 2v4"/>
  <path d="M18 2v4"/>
  <circle cx="9" cy="11" r="1.3" fill="currentColor"/>
  <path d="M9 11L12 20"/>
</svg>"##;

/// Modified Schiff — median origin shifted further, to the midpoint of the
/// last two tines (marked dot on the opposite side from Schiff).
pub const ICON_MODIFIED_SCHIFF: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 2v6c0 2 2 3 6 3s6-1 6-3V2"/>
  <path d="M6 2v4"/>
  <path d="M12 2v4"/>
  <path d="M18 2v4"/>
  <circle cx="15" cy="11" r="1.3" fill="currentColor"/>
  <path d="M15 11L12 20"/>
</svg>"##;

/// Inside Pitchfork — "projects backward through the handle": the median
/// continues (dashed) past the tine base instead of stopping there.
pub const ICON_INSIDE_PITCHFORK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 2v6c0 2 2 3 6 3s6-1 6-3V2"/>
  <path d="M6 2v4"/>
  <path d="M12 2v4"/>
  <path d="M18 2v4"/>
  <path d="M12 20V11"/>
  <path d="M12 11V2" stroke-dasharray="2 2"/>
</svg>"##;

/// Fixed-size Gann square (icon audit item 2 — was a byte-identical
/// duplicate of the resizable variant). A fixed 3x3 grid subdivision reads
/// as "equal fixed divisions", distinct from the resizable variant's
/// inscribed circle + X.
pub const ICON_GANN_SQUARE_FIXED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="4" y="4" width="16" height="16" rx="0.5"/>
  <path d="M4 4l16 16"/>
  <path d="M20 4L4 20"/>
  <path d="M12 4v16" stroke-dasharray="1.5 2"/>
  <path d="M4 12h16" stroke-dasharray="1.5 2"/>
  <rect x="8" y="8" width="8" height="8" fill="currentColor" opacity="0.15" stroke="none"/>
</svg>"##;

/// Redrawn 2026-08-07 (icon audit item 3): same 5-point count as XABCD (both
/// MultiPointFixed(5)) so dots alone don't distinguish them — the final
/// vertex (Cypher's defining C-D leg) is marked with a SQUARE instead of a
/// circle, an enclosing-form device rather than just a different squiggle.
pub const ICON_CYPHER_PATTERN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 8L7 18L11 10L16 19L21 3"/>
  <circle cx="2" cy="8" r="1.2" fill="currentColor"/><circle cx="7" cy="18" r="1.2" fill="currentColor"/><circle cx="11" cy="10" r="1.2" fill="currentColor"/><circle cx="16" cy="19" r="1.2" fill="currentColor"/>
  <rect x="19.5" y="1.5" width="3" height="3" fill="currentColor"/>
</svg>"##;

/// Harmonic gartley (mylittlechart).
pub const ICON_HARMONIC_GARTLEY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 19L8 5L12 13.5z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M12 13.5L15.5 8L21 16z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 19L8 5L12 13.5L15.5 8L21 16"/>
  <path d="M3 19L12 13.5" stroke-dasharray="1.5 1.5"/>
  <path d="M12 13.5L21 16" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Harmonic bat (mylittlechart).
pub const ICON_HARMONIC_BAT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 19L8 5L12 14.5z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M12 14.5L15.5 9L21 18z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 19L8 5L12 14.5L15.5 9L21 18"/>
  <path d="M3 19L12 14.5" stroke-dasharray="1.5 1.5"/>
  <path d="M12 14.5L21 18" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Harmonic alt bat (mylittlechart).
pub const ICON_HARMONIC_ALT_BAT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 18L8 5L12 15z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M12 15L15 10L21 20.5z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 18L8 5L12 15L15 10L21 20.5"/>
  <path d="M3 18L12 15" stroke-dasharray="1.5 1.5"/>
  <path d="M12 15L21 20.5" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Harmonic butterfly (mylittlechart).
pub const ICON_HARMONIC_BUTTERFLY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 16L8 5L12 14.5z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M12 14.5L15.5 8L21 20.5z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 16L8 5L12 14.5L15.5 8L21 20.5"/>
  <path d="M3 16L12 14.5" stroke-dasharray="1.5 1.5"/>
  <path d="M12 14.5L21 20.5" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Harmonic crab (mylittlechart).
pub const ICON_HARMONIC_CRAB: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 15L7.5 5L11 11z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M11 11L14.5 7L21 21z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 15L7.5 5L11 11L14.5 7L21 21"/>
  <path d="M3 15L11 11" stroke-dasharray="1.5 1.5"/>
  <path d="M11 11L21 21" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Harmonic deep crab (mylittlechart).
pub const ICON_HARMONIC_DEEP_CRAB: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 15L7.5 5L11 14z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M11 14L14.5 8L21 21z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 15L7.5 5L11 14L14.5 8L21 21"/>
  <path d="M3 15L11 14" stroke-dasharray="1.5 1.5"/>
  <path d="M11 14L21 21" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Harmonic shark (mylittlechart).
pub const ICON_HARMONIC_SHARK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 18L7 9L11 14z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M11 14L15 4L21 17z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 18L7 9L11 14L15 4L21 17"/>
  <path d="M3 18L11 14" stroke-dasharray="1.5 1.5"/>
  <path d="M11 14L21 17" stroke-dasharray="1.5 1.5"/>
</svg>"##;

/// Bars scan icon swap (A0, piece 3): the range-bracket-motif replacement
/// for `candle_pattern_scan`'s previous icon, matching the "scan tools
/// share a bracket motif" design note (`ICON_SMC_SCAN`'s own doc) that
/// `ICON_CANDLE_PATTERN_SCAN` predates. Wired to the `candle_pattern_scan`
/// toolbar row in `toolbar.rs`'s `pattern_section()` (piece 5) — the icon
/// constant name follows the new `bars_scan` id, not the old primitive name.
pub const ICON_BARS_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4.5 4H3v16h1.5"/>
  <path d="M19.5 4H21v16h-1.5"/>
  <rect x="9.75" y="5" width="4.5" height="14" rx="0.5" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M8 8v1.5"/>
  <path d="M8 15v1.5"/>
  <rect x="6.75" y="9.5" width="2.5" height="5.5"/>
  <path d="M12 6v2"/>
  <path d="M12 16v2"/>
  <rect x="10.75" y="8" width="2.5" height="8" fill="currentColor"/>
  <path d="M16 9v2"/>
  <path d="M16 15v1.5"/>
  <rect x="14.75" y="11" width="2.5" height="4"/>
</svg>"##;

/// Redrawn 2026-07-18 (owner-approved design round; see nemo/docs/mlc/research/icon-drafts/)
pub const ICON_ELLIOTT_IMPULSE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <polyline points="2,20 6,9 9,14 13,3 17,11 22,2"/>
  <circle cx="6" cy="9" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="9" cy="14" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="13" cy="3" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="17" cy="11" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="22" cy="2" r="1.2" fill="currentColor" stroke="none"/>
</svg>"##;

/// Redrawn 2026-07-18 (owner-approved design round; see nemo/docs/mlc/research/icon-drafts/)
pub const ICON_ELLIOTT_CORRECTION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <polyline points="2,6 8,16 13,9 22,20"/>
  <circle cx="8" cy="16" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="13" cy="9" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="22" cy="20" r="1.2" fill="currentColor" stroke="none"/>
</svg>"##;

/// Redrawn 2026-07-18 (owner-approved design round; see nemo/docs/mlc/research/icon-drafts/)
pub const ICON_ELLIOTT_TRIANGLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 4L21 11"/>
  <path d="M3 20L21 11"/>
  <polyline points="2,5 6,18 10,7 14,15 18,10" stroke-width="0.9"/>
</svg>"##;

/// Redrawn 2026-07-18 (owner-approved design round; see nemo/docs/mlc/research/icon-drafts/).
/// Generic base — used only when no double/triple distinction applies.
pub const ICON_ELLIOTT_COMBO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <polyline points="2,6 5,13 7,9 9,15 13,5 16,12 18,8 22,16"/>
  <circle cx="9" cy="15" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="13" cy="5" r="1.2" fill="currentColor" stroke="none"/>
  <circle cx="22" cy="16" r="1.2" fill="currentColor" stroke="none"/>
</svg>"##;

/// WXYXZ triple combination (icon audit item 2 — was byte-identical to the
/// double combo). Busier zigzag with 5 marked legs (W-X-Y-X-Z) — MORE
/// direction changes than the double combo, matching the extra W/X/Y/X/Z
/// structure.
pub const ICON_ELLIOTT_TRIPLE_COMBO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <polyline points="2,7 5,12 7,9 9,14 11,10 13,16 15,8 17,13 19,10 22,15"/>
  <circle cx="9" cy="14" r="1.1" fill="currentColor" stroke="none"/>
  <circle cx="13" cy="16" r="1.1" fill="currentColor" stroke="none"/>
  <circle cx="17" cy="13" r="1.1" fill="currentColor" stroke="none"/>
  <circle cx="22" cy="15" r="1.1" fill="currentColor" stroke="none"/>
</svg>"##;

/// Fixed volume profile: bars bounded by TWO vertical boundaries (left +
/// right) — a profile over a fixed, closed range.
pub const ICON_FIXED_VOLUME_PROFILE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 3v18"/><path d="M20 3v18"/>
  <rect x="4" y="6" width="8" height="3" fill="currentColor" fill-opacity="0.5"/>
  <rect x="4" y="11" width="14" height="3" fill="currentColor"/>
  <rect x="4" y="16" width="6" height="3" fill="currentColor" fill-opacity="0.3"/>
</svg>"##;

/// Anchored volume profile (icon audit item 2 — was byte-identical to the
/// fixed variant). ONE anchor point (marked dot + stem), bars growing
/// forward with no closing boundary — an open-ended profile from a single
/// anchor, not a bounded range.
pub const ICON_ANCHORED_VOLUME_PROFILE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="4" cy="4" r="1.5" fill="currentColor" stroke="none"/>
  <path d="M4 4v16"/>
  <rect x="4" y="7" width="8" height="3" fill="currentColor" fill-opacity="0.5"/>
  <rect x="4" y="11" width="14" height="3" fill="currentColor"/>
  <rect x="4" y="15" width="10" height="3" fill="currentColor" fill-opacity="0.5"/>
</svg>"##;

/// Anchored VWAP: anchor point (marked) + the VWAP line + dashed ±1/±2σ
/// deviation bands (matches the tooltip). Was wrongly aliased to the plain
/// Line Chart glyph (icon audit item 1 — unresolved, no distinguishing icon
/// at all before this).
pub const ICON_ANCHORED_VWAP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="4" cy="4" r="1.5" fill="currentColor" stroke="none"/>
  <path d="M4 4v8"/>
  <path d="M4 12h16"/>
  <path d="M4 7h16" stroke-dasharray="2 2" opacity="0.7"/>
  <path d="M4 17h16" stroke-dasharray="2 2" opacity="0.7"/>
</svg>"##;

/// Fixed VWAP: same VWAP line + dashed ±1/±2σ bands as
/// [`ICON_ANCHORED_VWAP`], but bounded by TWO vertical boundaries (left +
/// right) instead of one anchor point — the `ICON_FIXED_VOLUME_PROFILE` /
/// `ICON_ANCHORED_VOLUME_PROFILE` distinction, reapplied to VWAP.
pub const ICON_FIXED_VWAP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 3v18"/><path d="M20 3v18"/>
  <path d="M4 12h16"/>
  <path d="M4 7h16" stroke-dasharray="2 2" opacity="0.7"/>
  <path d="M4 17h16" stroke-dasharray="2 2" opacity="0.7"/>
</svg>"##;

/// A three-bar group with the middle bar picked out — the shape of a
/// claim about specific bars, which is what this primitive is.
pub const ICON_CANDLE_PATTERN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M5 7v10"/><rect x="3" y="9" width="4" height="6"/>
  <path d="M12 4v16"/><rect x="10" y="7" width="4" height="10" fill="currentColor"/>
  <path d="M19 7v10"/><rect x="17" y="9" width="4" height="6"/>
</svg>"##;

/// Candles under a lens — a survey of what the bars contain, as opposed
/// to [`ICON_CANDLE_PATTERN`]'s single named claim.
pub const ICON_CANDLE_PATTERN_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 8v8"/><rect x="2.5" y="10" width="3" height="4"/>
  <path d="M9 6v8"/><rect x="7.5" y="8" width="3" height="4"/>
  <circle cx="16" cy="12" r="5"/><path d="M19.5 15.5L22 18"/>
</svg>"##;

/// The opposing candle that a displacement left behind, drawn as a zone
/// with the leg breaking away from it.
pub const ICON_ORDER_BLOCK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="4" y="13" width="17" height="5" fill="currentColor" opacity="0.22" stroke="none"/>
  <path d="M6.5 10.5v2.5"/>
  <path d="M6.5 18v2"/>
  <rect x="4.75" y="13" width="3.5" height="5" fill="currentColor"/>
  <path d="M10 12l3.5-4 2.5 1.5 4-5.5"/>
  <path d="M17 4h3v3"/>
</svg>"##;

/// Two bars with an untouched band between them — the gap is the subject,
/// so the band is what the icon fills, not the bars.
pub const ICON_FAIR_VALUE_GAP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="3" y="9" width="18" height="4.5" fill="currentColor" opacity="0.22" stroke="none"/>
  <path d="M5.5 13.5v1.5"/>
  <path d="M5.5 19v1.5"/>
  <rect x="4" y="15" width="3" height="4"/>
  <path d="M12 3.5v2"/>
  <path d="M12 17.5v2"/>
  <rect x="10.5" y="5.5" width="3" height="12"/>
  <path d="M18.5 3v1.5"/>
  <path d="M18.5 7.5v1.5"/>
  <rect x="17" y="4.5" width="3" height="3"/>
</svg>"##;

/// A wick spiking through a level and a body closing back under it — the
/// whole distinction between a sweep and a break of structure in one shape.
pub const ICON_LIQUIDITY_SWEEP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 8.5h13" stroke-dasharray="2 2.5"/>
  <path d="M3 15l3-6.5 3 4.5 3-4.5 2 3"/>
  <path d="M18 3v7"/>
  <rect x="16.5" y="10" width="3" height="5" fill="currentColor"/>
  <path d="M18 15v2.5"/>
  <path d="M18 20.5l-1.5-1.5"/>
  <path d="M18 20.5l1.5-1.5"/>
</svg>"##;

/// A level with a body closing through it — the CLOSE is the event, so the
/// icon shows a body past the line rather than a wick.
pub const ICON_STRUCTURE_BREAK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M8 11h13" stroke-dasharray="2 2"/>
  <path d="M3 19l5-8 4 4.5 7-11"/>
  <circle cx="14.7" cy="11" r="1.7" fill="currentColor"/>
  <path d="M16.5 4.5h3v3"/>
</svg>"##;

/// A range halved at its equilibrium — the upper half shaded as premium,
/// the lower left open as discount.
pub const ICON_DEALING_RANGE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="4" y="4" width="16" height="8" fill="currentColor" opacity="0.18" stroke="none"/>
  <rect x="4" y="4" width="16" height="16" rx="1"/>
  <path d="M4 12h16" stroke-dasharray="2 2"/>
  <path d="M7 16.5l3-2 3 2.5 4-4"/>
</svg>"##;

/// A zone and a level under a lens — the smart-money survey, sharing
/// [`ICON_CANDLE_PATTERN_SCAN`]'s lens so the two read as one family.
pub const ICON_SMC_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4.5 4H3v16h1.5"/>
  <path d="M19.5 4H21v16h-1.5"/>
  <rect x="6" y="12.5" width="12" height="4.5" fill="currentColor" opacity="0.25" stroke="none"/>
  <path d="M8 10v2.5"/>
  <path d="M8 17v2"/>
  <rect x="6.5" y="12.5" width="3" height="4.5" fill="currentColor"/>
  <path d="M11 11l2.5-3 2 1.5 2.5-4"/>
</svg>"##;

/// The three phases in order: a flat accumulation range, a spike out of it
/// that closes back inside, and the leg away.
pub const ICON_AMD_CYCLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="2.5" y="9.5" width="7.5" height="5" rx="0.5" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 12.5c1-1.2 1.8 1.2 2.8 0s1.8 1.2 2.8 0"/>
  <path d="M9.5 12.5l2.5 7 3-9 6-6.5"/>
  <path d="M17.5 4h3.5v3.5"/>
</svg>"##;

/// A 5-wave impulse leg under the bracket scan-motif, five dotted pivots
/// marking each wave's own endpoint — `icons_v4.json` batch B's own
/// `elliott_scan` body (2026-09-23 toolbar icons, drawn ahead of A2, wired
/// here), sharing `ICON_SMC_SCAN`'s own bracket wrapper so every autodetector
/// reads as one family.
pub const ICON_ELLIOTT_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4.5 4H3v16h1.5"/>
  <path d="M19.5 4H21v16h-1.5"/>
  <path d="M6 17l2.5-4.5 2 2 3-6 2 3 2.5-4"/>
  <circle cx="8.5" cy="12.5" r="1.2" fill="currentColor"/>
  <circle cx="10.5" cy="14.5" r="1.2" fill="currentColor"/>
  <circle cx="13.5" cy="8.5" r="1.2" fill="currentColor"/>
  <circle cx="15.5" cy="11.5" r="1.2" fill="currentColor"/>
  <circle cx="18" cy="7.5" r="1.2" fill="currentColor"/>
</svg>"##;

/// The 6-point `0-X-A-B-C-D` zigzag Duddella's own "5-0" names, with the
/// pattern's own completion triangle tinted — `icons_v4.json` batch B's own
/// `harmonic_five_o` body (2026-09-23 toolbar icons, drawn ahead of A3,
/// wired here), the manual counterpart to `ICON_HARMONIC_SCAN` (A3.5).
pub const ICON_HARMONIC_FIVE_O: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2.5 6L6 17l4-6 4 8 4-13 3.5 7"/>
  <path d="M10 11L18 6L21.5 13z" fill="currentColor" opacity="0.18" stroke="none"/>
  <circle cx="2.5" cy="6" r="1.2" fill="currentColor"/>
  <circle cx="6" cy="17" r="1.2" fill="currentColor"/>
  <circle cx="10" cy="11" r="1.2" fill="currentColor"/>
  <circle cx="14" cy="19" r="1.2" fill="currentColor"/>
  <circle cx="18" cy="6" r="1.2" fill="currentColor"/>
  <circle cx="21.5" cy="13" r="1.2" fill="currentColor"/>
</svg>"##;

/// The bracket scan-motif around a 5-0-shaped leg, tinted — `icons_v4.json`
/// batch B's own `harmonic_scan` body, sharing `ICON_SMC_SCAN`'s/
/// `ICON_ELLIOTT_SCAN`'s own bracket wrapper so every autodetector reads as
/// one family.
pub const ICON_HARMONIC_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4.5 4H3v16h1.5"/>
  <path d="M19.5 4H21v16h-1.5"/>
  <path d="M6 17L9 7l2.5 5 2.5-3.5L18 17z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M6 17L9 7l2.5 5 2.5-3.5L18 17"/>
</svg>"##;

/// The bracket scan-motif around a stack of tinted horizontal Fib-level
/// bands — `icons_v4.json` batch B's own `fib_scan` body (2026-09-23
/// toolbar icons, drawn ahead of A4, wired here), sharing `ICON_SMC_SCAN`'s/
/// `ICON_ELLIOTT_SCAN`'s/`ICON_HARMONIC_SCAN`'s own bracket wrapper so every
/// autodetector reads as one family.
pub const ICON_FIB_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4.5 4H3v16h1.5"/>
  <path d="M19.5 4H21v16h-1.5"/>
  <rect x="6" y="6" width="12" height="3.5" fill="currentColor" opacity="0.18" stroke="none"/>
  <rect x="6" y="14.5" width="12" height="3.5" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M6 6h12"/>
  <path d="M6 9.5h12"/>
  <path d="M6 14.5h12"/>
  <path d="M6 18h12"/>
</svg>"##;

/// Two columns of unequal cells facing each other across a bar's centre —
/// the bid-versus-ask reading a footprint cell carries.
pub const ICON_IMBALANCE_ZONE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M12 3.5v17"/>
  <rect x="4" y="5" width="6" height="3.5" rx="0.5" fill="currentColor" opacity="0.35" stroke="none"/>
  <rect x="4" y="5" width="6" height="3.5" rx="0.5"/>
  <rect x="14" y="5" width="3" height="3.5" rx="0.5"/>
  <rect x="6" y="10.25" width="4" height="3.5" rx="0.5"/>
  <rect x="14" y="10.25" width="6" height="3.5" rx="0.5" fill="currentColor" opacity="0.35" stroke="none"/>
  <rect x="14" y="10.25" width="6" height="3.5" rx="0.5"/>
  <rect x="7" y="15.5" width="3" height="3.5" rx="0.5"/>
  <rect x="14" y="15.5" width="4" height="3.5" rx="0.5"/>
</svg>"##;

/// Volume arriving into a level that does not move — arrows into a wall,
/// which is what absorption is.
pub const ICON_ABSORPTION_MARK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 13h18" stroke-width="2.5"/>
  <path d="M7 4v6"/>
  <path d="M5 8l2 2 2-2"/>
  <path d="M12 4v6"/>
  <path d="M10 8l2 2 2-2"/>
  <path d="M17 4v6"/>
  <path d="M15 8l2 2 2-2"/>
  <path d="M4 17.5h16" stroke-dasharray="2 2"/>
</svg>"##;

/// Price and delta pointing opposite ways — two legs from one origin,
/// which is the entire claim the mark makes.
pub const ICON_DELTA_DIVERGENCE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 11l4-3 4 2 4-5 6 1"/>
  <path d="M3 15l4 1 4-1.5 4 3.5 6 1.5" stroke-dasharray="2 2"/>
  <circle cx="15" cy="5" r="1.4" fill="currentColor"/>
  <circle cx="15" cy="18.5" r="1.4" fill="currentColor"/>
</svg>"##;

/// A footprint grid under the family lens — same lens as the other two
/// scanners, different subject beneath it.
pub const ICON_ORDER_FLOW_SCAN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4.5 4H3v16h1.5"/>
  <path d="M19.5 4H21v16h-1.5"/>
  <path d="M12 5v14"/>
  <rect x="6" y="6" width="4.5" height="3.5" rx="0.5" fill="currentColor" opacity="0.35" stroke="none"/>
  <rect x="6" y="6" width="4.5" height="3.5" rx="0.5"/>
  <rect x="13.5" y="6" width="3" height="3.5" rx="0.5"/>
  <rect x="7.5" y="14.5" width="3" height="3.5" rx="0.5"/>
  <rect x="13.5" y="14.5" width="4.5" height="3.5" rx="0.5" fill="currentColor" opacity="0.35" stroke="none"/>
  <rect x="13.5" y="14.5" width="4.5" height="3.5" rx="0.5"/>
</svg>"##;

/// Trading slot — briefcase glyph (hosts `TradingContainer`). Owner-approved
/// final design 2026-07-11.
pub const ICON_SLOT_TRADING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="3" y="7" width="18" height="13" rx="2"/>
  <path d="M8 7V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
  <path d="M3 13h18"/>
</svg>"##;

/// DOM slot glyph variant (redrawn 2026-07-18; kept as a variant per
/// owner order 2026-08-01 — the ACTIVE rail badge is the text chip
/// below, «текущие скины можно оставить просто как варианты»).
pub const ICON_SLOT_DOM_GLYPH: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="10" y="3" width="4" height="18" rx="0.5"/>
  <line x1="2" y1="6" x2="10" y2="6"/>
  <line x1="2" y1="10" x2="10" y2="10"/>
  <line x1="5" y1="14" x2="10" y2="14"/>
  <line x1="6" y1="18" x2="10" y2="18"/>
  <line x1="14" y1="6" x2="18" y2="6"/>
  <line x1="14" y1="10" x2="22" y2="10"/>
  <line x1="14" y1="14" x2="19" y2="14"/>
  <line x1="14" y1="18" x2="22" y2="18"/>
</svg>"##;

/// DOM slot — text chip "DOM" (owner order 2026-08-01 «нужно написать
/// DOM TT L2 BT»; same chip grammar as the L2 badge).
pub const ICON_SLOT_DOM: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M12 3v18"/>
  <path d="M13.5 5h6"/>
  <path d="M13.5 8h4"/>
  <path d="M13.5 11h7"/>
  <path d="M3.5 13h7"/>
  <path d="M5.5 16h5"/>
  <path d="M7.5 19h3"/>
</svg>"##;

/// Trade Tape slot glyph variant (redrawn 2026-07-18; kept as a variant
/// per owner order 2026-08-01).
pub const ICON_SLOT_TAPE_GLYPH: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <line x1="4" y1="2" x2="20" y2="2" stroke-width="0.9"/>
  <line x1="5" y1="7" x2="17" y2="7"/>
  <line x1="7" y1="11" x2="20" y2="11"/>
  <line x1="5" y1="15" x2="14" y2="15"/>
  <line x1="7" y1="19" x2="18" y2="19"/>
  <line x1="4" y1="22" x2="20" y2="22" stroke-width="0.9"/>
</svg>"##;

/// Trade Tape slot — text chip "TT" (owner order 2026-08-01).
pub const ICON_SLOT_TAPE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3.5 7.5L5 5l1.5 2.5z" fill="currentColor"/>
  <path d="M9 6.25h11"/>
  <path d="M3.5 10.5L5 13l1.5-2.5z" fill="currentColor"/>
  <path d="M9 11.75h7"/>
  <path d="M3.5 18.5L5 16l1.5 2.5z" fill="currentColor"/>
  <path d="M9 17.25h9"/>
</svg>"##;

/// L2 Tape slot — text-chip glyph. Redrawn 2026-07-18 (owner-picked
/// chip design; see nemo/docs/mlc/research/icon-drafts/); previous design
/// owner-picked (round 4, L-b) 2026-07-11.
pub const ICON_SLOT_L2: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2.5 20V7h3v4h3v4h3v5z" fill="currentColor" opacity="0.2" stroke="none"/>
  <path d="M12.5 20v-5h3v-4h3v-4h3v13z" fill="currentColor" opacity="0.2" stroke="none"/>
  <path d="M2.5 20V7h3v4h3v4h3v5"/>
  <path d="M12.5 20v-5h3v-4h3v-4h3v13"/>
  <path d="M2 20.5h20"/>
</svg>"##;

/// Big Trades slot glyph variant — sonar/ripple (owner-picked #12 of 24,
/// 2026-07-11; kept as a variant per owner order 2026-08-01).
pub const ICON_SLOT_BIG_TRADES_GLYPH: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="12" cy="12" r="2" fill="currentColor" stroke="none"/>
  <path d="M12 5a7 7 0 0 1 7 7"/>
  <path d="M12 1.5A10.5 10.5 0 0 1 22.5 12"/>
  <path d="M12 19a7 7 0 0 1-7-7"/>
  <path d="M12 22.5A10.5 10.5 0 0 1 1.5 12"/>
</svg>"##;

/// Big Trades slot — text chip "BT" (owner order 2026-08-01).
pub const ICON_SLOT_BIG_TRADES: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 4v16.5h17.5"/>
  <path d="M6.5 16l4-4.5 3 2.5 6-7"/>
  <path d="M16.5 7h3v3"/>
</svg>"##;

/// Liquidations slot chip — "LIQ" in the slot text-chip grammar
/// (owner spec 2026-08-01, plan D: rail button under BT).
pub const ICON_SLOT_LIQ: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M13.5 2.5L5 13.5h6.5L10 21.5l9-11.5h-6.5z" fill="currentColor" opacity="0.2" stroke="none"/>
  <path d="M13.5 2.5L5 13.5h6.5L10 21.5l9-11.5h-6.5z"/>
</svg>"##;

/// Family ewa (mylittlechart).
pub const ICON_FAMILY_EWA: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 19l4-7 3 3 5-10 3 5 3-4"/>
  <circle cx="7" cy="12" r="1.2" fill="currentColor"/>
  <circle cx="10" cy="15" r="1.2" fill="currentColor"/>
  <circle cx="15" cy="5" r="1.2" fill="currentColor"/>
  <circle cx="18" cy="10" r="1.2" fill="currentColor"/>
  <circle cx="21" cy="6" r="1.2" fill="currentColor"/>
</svg>"##;

/// Family bars (mylittlechart).
pub const ICON_FAMILY_BARS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="9.25" y="3" width="5.5" height="18" rx="1" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M5.5 7v2"/>
  <path d="M5.5 16v2"/>
  <rect x="4" y="9" width="3" height="7"/>
  <path d="M12 4.5v3"/>
  <path d="M12 17v2.5"/>
  <rect x="10.5" y="7.5" width="3" height="9.5" fill="currentColor"/>
  <path d="M18.5 9v2"/>
  <path d="M18.5 17v2"/>
  <rect x="17" y="11" width="3" height="6"/>
</svg>"##;

/// Family smc (mylittlechart).
pub const ICON_FAMILY_SMC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <rect x="4" y="13" width="17" height="5" fill="currentColor" opacity="0.22" stroke="none"/>
  <path d="M6.5 10.5v2.5"/>
  <path d="M6.5 18v2"/>
  <rect x="4.75" y="13" width="3.5" height="5" fill="currentColor"/>
  <path d="M10 12l3.5-4 2.5 1.5 4-5.5"/>
  <path d="M17 4h3v3"/>
</svg>"##;

/// Family order flow (mylittlechart).
pub const ICON_FAMILY_ORDER_FLOW: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M12 3.5v17"/>
  <rect x="5" y="5" width="5" height="3.5" rx="0.5"/>
  <rect x="14" y="5" width="3" height="3.5" rx="0.5"/>
  <rect x="4" y="10.25" width="6" height="3.5" rx="0.5" fill="currentColor"/>
  <rect x="14" y="10.25" width="6" height="3.5" rx="0.5"/>
  <rect x="7" y="15.5" width="3" height="3.5" rx="0.5"/>
  <rect x="14" y="15.5" width="5" height="3.5" rx="0.5"/>
</svg>"##;

/// Family harmonics (mylittlechart).
pub const ICON_FAMILY_HARMONICS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M3 18L8 5l4 8z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M12 13l4-5 5 11z" fill="currentColor" opacity="0.18" stroke="none"/>
  <path d="M3 18L8 5l4 8 4-5 5 11"/>
  <path d="M3 18l9-5" stroke-dasharray="2 2"/>
  <path d="M12 13l9 6" stroke-dasharray="2 2"/>
</svg>"##;

/// Family classic (mylittlechart).
pub const ICON_FAMILY_CLASSIC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 18l3-6 3 3.5 4-10 4 10 3-3.5 3 6"/>
  <path d="M4 15.5h16" stroke-dasharray="2 2"/>
</svg>"##;

/// Every icon here with its name (a gallery, a picker); naming one const instead
/// keeps the rest out of the binary.
pub const ALL: &[(&str, &str)] = &[
    ("range_bar", ICON_RANGE_BAR),
    ("volume_bar", ICON_VOLUME_BAR),
    ("dollar_bar", ICON_DOLLAR_BAR),
    ("tick_bar", ICON_TICK_BAR),
    ("tick_imbalance_bar", ICON_TICK_IMBALANCE_BAR),
    ("volume_imbalance_bar", ICON_VOLUME_IMBALANCE_BAR),
    ("run_bar", ICON_RUN_BAR),
    ("renko", ICON_RENKO),
    ("pnf", ICON_PNF),
    ("kagi", ICON_KAGI),
    ("three_line_break", ICON_THREE_LINE_BREAK),
    ("footprint_cells", ICON_FOOTPRINT_CELLS),
    ("tpo", ICON_TPO),
    ("ray_left", ICON_RAY_LEFT),
    ("horizontal_ray_left", ICON_HORIZONTAL_RAY_LEFT),
    ("schiff_pitchfork", ICON_SCHIFF_PITCHFORK),
    ("modified_schiff", ICON_MODIFIED_SCHIFF),
    ("inside_pitchfork", ICON_INSIDE_PITCHFORK),
    ("gann_square_fixed", ICON_GANN_SQUARE_FIXED),
    ("cypher_pattern", ICON_CYPHER_PATTERN),
    ("harmonic_gartley", ICON_HARMONIC_GARTLEY),
    ("harmonic_bat", ICON_HARMONIC_BAT),
    ("harmonic_alt_bat", ICON_HARMONIC_ALT_BAT),
    ("harmonic_butterfly", ICON_HARMONIC_BUTTERFLY),
    ("harmonic_crab", ICON_HARMONIC_CRAB),
    ("harmonic_deep_crab", ICON_HARMONIC_DEEP_CRAB),
    ("harmonic_shark", ICON_HARMONIC_SHARK),
    ("bars_scan", ICON_BARS_SCAN),
    ("elliott_impulse", ICON_ELLIOTT_IMPULSE),
    ("elliott_correction", ICON_ELLIOTT_CORRECTION),
    ("elliott_triangle", ICON_ELLIOTT_TRIANGLE),
    ("elliott_combo", ICON_ELLIOTT_COMBO),
    ("elliott_triple_combo", ICON_ELLIOTT_TRIPLE_COMBO),
    ("fixed_volume_profile", ICON_FIXED_VOLUME_PROFILE),
    ("anchored_volume_profile", ICON_ANCHORED_VOLUME_PROFILE),
    ("anchored_vwap", ICON_ANCHORED_VWAP),
    ("fixed_vwap", ICON_FIXED_VWAP),
    ("candle_pattern", ICON_CANDLE_PATTERN),
    ("candle_pattern_scan", ICON_CANDLE_PATTERN_SCAN),
    ("order_block", ICON_ORDER_BLOCK),
    ("fair_value_gap", ICON_FAIR_VALUE_GAP),
    ("liquidity_sweep", ICON_LIQUIDITY_SWEEP),
    ("structure_break", ICON_STRUCTURE_BREAK),
    ("dealing_range", ICON_DEALING_RANGE),
    ("smc_scan", ICON_SMC_SCAN),
    ("amd_cycle", ICON_AMD_CYCLE),
    ("elliott_scan", ICON_ELLIOTT_SCAN),
    ("harmonic_five_o", ICON_HARMONIC_FIVE_O),
    ("harmonic_scan", ICON_HARMONIC_SCAN),
    ("fib_scan", ICON_FIB_SCAN),
    ("imbalance_zone", ICON_IMBALANCE_ZONE),
    ("absorption_mark", ICON_ABSORPTION_MARK),
    ("delta_divergence", ICON_DELTA_DIVERGENCE),
    ("order_flow_scan", ICON_ORDER_FLOW_SCAN),
    ("slot_trading", ICON_SLOT_TRADING),
    ("slot_dom_glyph", ICON_SLOT_DOM_GLYPH),
    ("slot_dom", ICON_SLOT_DOM),
    ("slot_tape_glyph", ICON_SLOT_TAPE_GLYPH),
    ("slot_tape", ICON_SLOT_TAPE),
    ("slot_l2", ICON_SLOT_L2),
    ("slot_big_trades_glyph", ICON_SLOT_BIG_TRADES_GLYPH),
    ("slot_big_trades", ICON_SLOT_BIG_TRADES),
    ("slot_liq", ICON_SLOT_LIQ),
    ("family_ewa", ICON_FAMILY_EWA),
    ("family_bars", ICON_FAMILY_BARS),
    ("family_smc", ICON_FAMILY_SMC),
    ("family_order_flow", ICON_FAMILY_ORDER_FLOW),
    ("family_harmonics", ICON_FAMILY_HARMONICS),
    ("family_classic", ICON_FAMILY_CLASSIC),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::assets::icons::test_support::assert_root_stroke_width_is_thin;

    #[test]
    fn root_stroke_width_stays_thin() {
        assert_eq!(ALL.len(), 69, "the gallery lists them all");
        let icons: &[(&str, &str)] = &[
            ("ICON_RANGE_BAR", ICON_RANGE_BAR),
            ("ICON_VOLUME_BAR", ICON_VOLUME_BAR),
            ("ICON_DOLLAR_BAR", ICON_DOLLAR_BAR),
            ("ICON_TICK_BAR", ICON_TICK_BAR),
            ("ICON_TICK_IMBALANCE_BAR", ICON_TICK_IMBALANCE_BAR),
            ("ICON_VOLUME_IMBALANCE_BAR", ICON_VOLUME_IMBALANCE_BAR),
            ("ICON_RUN_BAR", ICON_RUN_BAR),
            ("ICON_RENKO", ICON_RENKO),
            ("ICON_PNF", ICON_PNF),
            ("ICON_KAGI", ICON_KAGI),
            ("ICON_THREE_LINE_BREAK", ICON_THREE_LINE_BREAK),
            ("ICON_FOOTPRINT_CELLS", ICON_FOOTPRINT_CELLS),
            ("ICON_TPO", ICON_TPO),
            ("ICON_RAY_LEFT", ICON_RAY_LEFT),
            ("ICON_HORIZONTAL_RAY_LEFT", ICON_HORIZONTAL_RAY_LEFT),
            ("ICON_SCHIFF_PITCHFORK", ICON_SCHIFF_PITCHFORK),
            ("ICON_MODIFIED_SCHIFF", ICON_MODIFIED_SCHIFF),
            ("ICON_INSIDE_PITCHFORK", ICON_INSIDE_PITCHFORK),
            ("ICON_GANN_SQUARE_FIXED", ICON_GANN_SQUARE_FIXED),
            ("ICON_CYPHER_PATTERN", ICON_CYPHER_PATTERN),
            ("ICON_HARMONIC_GARTLEY", ICON_HARMONIC_GARTLEY),
            ("ICON_HARMONIC_BAT", ICON_HARMONIC_BAT),
            ("ICON_HARMONIC_ALT_BAT", ICON_HARMONIC_ALT_BAT),
            ("ICON_HARMONIC_BUTTERFLY", ICON_HARMONIC_BUTTERFLY),
            ("ICON_HARMONIC_CRAB", ICON_HARMONIC_CRAB),
            ("ICON_HARMONIC_DEEP_CRAB", ICON_HARMONIC_DEEP_CRAB),
            ("ICON_HARMONIC_SHARK", ICON_HARMONIC_SHARK),
            ("ICON_BARS_SCAN", ICON_BARS_SCAN),
            ("ICON_ELLIOTT_IMPULSE", ICON_ELLIOTT_IMPULSE),
            ("ICON_ELLIOTT_CORRECTION", ICON_ELLIOTT_CORRECTION),
            ("ICON_ELLIOTT_TRIANGLE", ICON_ELLIOTT_TRIANGLE),
            ("ICON_ELLIOTT_COMBO", ICON_ELLIOTT_COMBO),
            ("ICON_ELLIOTT_TRIPLE_COMBO", ICON_ELLIOTT_TRIPLE_COMBO),
            ("ICON_FIXED_VOLUME_PROFILE", ICON_FIXED_VOLUME_PROFILE),
            ("ICON_ANCHORED_VOLUME_PROFILE", ICON_ANCHORED_VOLUME_PROFILE),
            ("ICON_ANCHORED_VWAP", ICON_ANCHORED_VWAP),
            ("ICON_FIXED_VWAP", ICON_FIXED_VWAP),
            ("ICON_CANDLE_PATTERN", ICON_CANDLE_PATTERN),
            ("ICON_CANDLE_PATTERN_SCAN", ICON_CANDLE_PATTERN_SCAN),
            ("ICON_ORDER_BLOCK", ICON_ORDER_BLOCK),
            ("ICON_FAIR_VALUE_GAP", ICON_FAIR_VALUE_GAP),
            ("ICON_LIQUIDITY_SWEEP", ICON_LIQUIDITY_SWEEP),
            ("ICON_STRUCTURE_BREAK", ICON_STRUCTURE_BREAK),
            ("ICON_DEALING_RANGE", ICON_DEALING_RANGE),
            ("ICON_SMC_SCAN", ICON_SMC_SCAN),
            ("ICON_AMD_CYCLE", ICON_AMD_CYCLE),
            ("ICON_ELLIOTT_SCAN", ICON_ELLIOTT_SCAN),
            ("ICON_HARMONIC_FIVE_O", ICON_HARMONIC_FIVE_O),
            ("ICON_HARMONIC_SCAN", ICON_HARMONIC_SCAN),
            ("ICON_FIB_SCAN", ICON_FIB_SCAN),
            ("ICON_IMBALANCE_ZONE", ICON_IMBALANCE_ZONE),
            ("ICON_ABSORPTION_MARK", ICON_ABSORPTION_MARK),
            ("ICON_DELTA_DIVERGENCE", ICON_DELTA_DIVERGENCE),
            ("ICON_ORDER_FLOW_SCAN", ICON_ORDER_FLOW_SCAN),
            ("ICON_SLOT_TRADING", ICON_SLOT_TRADING),
            ("ICON_SLOT_DOM_GLYPH", ICON_SLOT_DOM_GLYPH),
            ("ICON_SLOT_DOM", ICON_SLOT_DOM),
            ("ICON_SLOT_TAPE_GLYPH", ICON_SLOT_TAPE_GLYPH),
            ("ICON_SLOT_TAPE", ICON_SLOT_TAPE),
            ("ICON_SLOT_L2", ICON_SLOT_L2),
            ("ICON_SLOT_BIG_TRADES_GLYPH", ICON_SLOT_BIG_TRADES_GLYPH),
            ("ICON_SLOT_BIG_TRADES", ICON_SLOT_BIG_TRADES),
            ("ICON_SLOT_LIQ", ICON_SLOT_LIQ),
            ("ICON_FAMILY_EWA", ICON_FAMILY_EWA),
            ("ICON_FAMILY_BARS", ICON_FAMILY_BARS),
            ("ICON_FAMILY_SMC", ICON_FAMILY_SMC),
            ("ICON_FAMILY_ORDER_FLOW", ICON_FAMILY_ORDER_FLOW),
            ("ICON_FAMILY_HARMONICS", ICON_FAMILY_HARMONICS),
            ("ICON_FAMILY_CLASSIC", ICON_FAMILY_CLASSIC),
        ];
        for (name, svg) in icons {
            assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"), "{name} is one svg");
            assert_root_stroke_width_is_thin(name, svg);
        }
    }
}
