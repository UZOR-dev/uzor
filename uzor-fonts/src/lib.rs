//! Bundled font assets for the uzor UI framework.
//!
//! All fonts are embedded as `&[u8]` constants so they are available at
//! compile time without any file-system access at runtime.
//!
//! # Families
//!
//! - **Roboto** — default UI sans-serif (regular, bold, italic, bold-italic)
//! - **PT Root UI** — variable-weight sans-serif from Paratype (single VF file)
//! - **JetBrains Mono** — monospace with full Unicode box-drawing coverage
//! - **Symbols Nerd Font Mono** — Powerline / Nerd Font PUA / dev icons
//! - **Noto Sans Symbols 2** — wide symbol / mathematical coverage
//! - **Noto Color Emoji** — color emoji (COLRv1/v0 + CBDT bitmaps; via the uzor-fonts-emoji crate)
//! - **Noto Emoji** — color-neutral emoji fallback (legacy, all backends)
//! - **Noto Sans CJK SC** — CJK Simplified Chinese + kana + Hangul (via the uzor-fonts-cjk-a/-b crates)
//! - **Noto Sans Arabic** — Arabic script
//! - **Noto Sans Devanagari** — Devanagari / Hindi script

// ── Roboto ────────────────────────────────────────────────────────────────────

/// Roboto Regular (400).
pub static ROBOTO_REGULAR: &[u8] = include_bytes!("../fonts/Roboto-Regular.ttf");

/// Roboto Bold (700).
pub static ROBOTO_BOLD: &[u8] = include_bytes!("../fonts/Roboto-Bold.ttf");

/// Roboto Italic (400 italic).
pub static ROBOTO_ITALIC: &[u8] = include_bytes!("../fonts/Roboto-Italic.ttf");

/// Roboto Bold Italic (700 italic).
pub static ROBOTO_BOLD_ITALIC: &[u8] = include_bytes!("../fonts/Roboto-BoldItalic.ttf");

// ── PT Root UI ────────────────────────────────────────────────────────────────
//
// Variable font — a single file covers the full weight axis (100–900).
// Pass it to fontdue as-is; rasterisation at any weight works from one binary.

/// PT Root UI Variable Font (covers weight axis 100–900).
pub static PT_ROOT_UI_VF: &[u8] = include_bytes!("../fonts/PTRootUI-VF.ttf");

// ── Inter ─────────────────────────────────────────────────────────────────────
//
// Inter (OFL-1.1, rsms/inter, see fonts/Inter-OFL.txt): static instances cut from
// the variable font at opsz=14 (Chrome's optical size for UI text) for the
// weights CSS UIs use. fontdue rasterises static faces only, so each weight is
// its own file; `inter_for_weight` picks the nearest.

/// Inter 400.
pub static INTER_400: &[u8] = include_bytes!("../fonts/Inter-400.ttf");
/// Inter 500.
pub static INTER_500: &[u8] = include_bytes!("../fonts/Inter-500.ttf");
/// Inter 550.
pub static INTER_550: &[u8] = include_bytes!("../fonts/Inter-550.ttf");
/// Inter 600.
pub static INTER_600: &[u8] = include_bytes!("../fonts/Inter-600.ttf");
/// Inter 650.
pub static INTER_650: &[u8] = include_bytes!("../fonts/Inter-650.ttf");
/// Inter 700.
pub static INTER_700: &[u8] = include_bytes!("../fonts/Inter-700.ttf");

/// Bundled Inter weights, ascending.
pub const INTER_WEIGHTS: [u16; 6] = [400, 500, 550, 600, 650, 700];

/// Inter bytes for the nearest bundled weight to `w` (CSS 100–900).
pub fn inter_for_weight(w: u16) -> &'static [u8] {
    let i = INTER_WEIGHTS.iter().enumerate().min_by_key(|(_, &x)| (x as i32 - w as i32).abs()).map(|(i, _)| i).unwrap_or(0);
    [INTER_400, INTER_500, INTER_550, INTER_600, INTER_650, INTER_700][i]
}

// ── JetBrains Mono ────────────────────────────────────────────────────────────
//
// Monospace font with full Unicode box-drawing coverage (U+2500–U+259F).
// Used for terminal-style rendering (PTY output, code blocks, etc.).

/// JetBrains Mono Regular — monospace with box-drawing support.
pub static JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../fonts/JetBrainsMono-Regular.ttf");

/// JetBrains Mono Bold.
pub static JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");

// ── Source Serif 4 ────────────────────────────────────────────────────────────
//
// OFL text serif (adobe-fonts/source-serif). Used when a preset rebinds the
// text role. The face name inside the file is "Source Serif 4".

/// Source Serif 4 Regular.
pub static SOURCE_SERIF_4_REGULAR: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.ttf");

/// Source Serif 4 Bold.
pub static SOURCE_SERIF_4_BOLD: &[u8] = include_bytes!("../fonts/SourceSerif4-Bold.ttf");

/// Source Serif 4 Italic.
pub static SOURCE_SERIF_4_ITALIC: &[u8] = include_bytes!("../fonts/SourceSerif4-It.ttf");

/// Source Serif 4 Bold Italic.
pub static SOURCE_SERIF_4_BOLD_ITALIC: &[u8] = include_bytes!("../fonts/SourceSerif4-BoldIt.ttf");

// ── Fallback fonts ────────────────────────────────────────────────────────────

/// Symbols Nerd Font Mono — Powerline separators (U+E0B0–U+E0B3), Nerd Font PUA
/// (U+E000–U+F8FF), dev icons, and wide Unicode symbol coverage.
pub static SYMBOLS_NERD_FONT_MONO: &[u8] =
    include_bytes!("../fonts/SymbolsNerdFontMono-Regular.ttf");

/// Noto Sans Symbols 2 — wide symbol / mathematical coverage.
pub static NOTO_SANS_SYMBOLS2: &[u8] =
    include_bytes!("../fonts/NotoSansSymbols2-Regular.ttf");

/// Noto Color Emoji — color emoji font containing both COLRv1/v0 vectors and CBDT bitmaps.
/// Supported natively by vello-gpu (skrifa 0.40+); other backends fall through
/// to the monochrome outline or skip to the next fallback. Comes from the
/// `uzor-fonts-emoji` crate via build.rs (too big for this crate's upload).
pub static NOTO_COLOR_EMOJI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/NotoColorEmoji.ttf"));

// ── i18n script fonts ─────────────────────────────────────────────────────────
//
// Arabic and Devanagari are bundled. CJK SC (like Noto Color Emoji above) is too big for
// this crate's crates.io upload: build.rs copies it into OUT_DIR from the `uzor-fonts-cjk-a`
// and `-b` build-dependency crates (two halves), and it is included from there.

/// Noto Sans CJK SC Regular — covers Simplified Chinese (zh), Japanese kana
/// (ja), and Korean Hangul (ko). ~16 MB; from the uzor-fonts-cjk-a/-b crates via build.rs.
pub static NOTO_SANS_CJK_SC: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/NotoSansCJKsc-Regular.otf"));

/// Noto Sans Arabic Regular — covers Arabic script (ar). ~170 KB; bundled.
pub static NOTO_SANS_ARABIC: &[u8] = include_bytes!("../fonts/NotoSansArabic-Regular.ttf");

/// Noto Sans Devanagari Regular — covers Devanagari script (hi). ~157 KB; bundled.
pub static NOTO_SANS_DEVANAGARI: &[u8] = include_bytes!("../fonts/NotoSansDevanagari-Regular.ttf");

/// DejaVu Sans Regular — broad BMP coverage. Fills the gap left by subsetted
/// Roboto (no arrows / dingbats / geometric shapes) and NotoSansSymbols2
/// (mostly supplementary U+1xxxx symbols).
///
/// Covers: Arrows (U+2190–21FF), General Punctuation, Math Operators,
/// Geometric Shapes (U+25A0–25FF), Box Drawing, Dingbats (U+2700–27BF, partial),
/// Letterlike Symbols. Bundled directly (~757 KB) — small enough not to need a
/// separate download.
pub static DEJAVU_SANS: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");

/// Noto Emoji — color-neutral emoji coverage (legacy, works on all backends).
pub static NOTO_EMOJI: &[u8] = include_bytes!("../fonts/NotoEmoji-Regular.ttf");

#[cfg(test)]
mod tests {
    use super::*;

    /// The fonts that used to be downloaded arrive whole (sfnt magic + exact size).
    #[test]
    fn big_and_i18n_fonts_are_complete() {
        for (name, bytes, len, magic) in [
            ("emoji", NOTO_COLOR_EMOJI, 10_673_480, &[0u8, 1, 0, 0][..]),
            ("cjk", NOTO_SANS_CJK_SC, 16_437_364, b"OTTO"),
            ("arabic", NOTO_SANS_ARABIC, 170_196, &[0, 1, 0, 0]),
            ("devanagari", NOTO_SANS_DEVANAGARI, 157_068, &[0, 1, 0, 0]),
        ] {
            assert_eq!(bytes.len(), len, "{name}");
            assert_eq!(&bytes[..4], magic, "{name}");
        }
    }
}
