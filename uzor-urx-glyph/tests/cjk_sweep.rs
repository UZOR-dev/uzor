//! Heavy CJK raster sweep — fractional pixel sizes (CSS emits fractional
//! px) and the whole CJK Unified Ideographs block, hunting the glyph/size
//! combination that panics the swash rasteriser. Run with `--ignored`.

use swash::FontRef;
use uzor_urx_glyph::{rasterise_glyph, register_font};

fn cjk_font() -> (uzor_urx_core::scene::FontId, Vec<u8>) {
    let bytes = uzor_fonts::NOTO_SANS_CJK_SC.to_vec();
    let id = register_font(bytes.clone()).expect("register CJK face");
    (id, bytes)
}

/// The 15 native language names of the us General tab (mlc-core i18n),
/// at the fractional sizes CSS layout actually emits.
#[test]
fn native_language_names_at_fractional_sizes() {
    let (font, bytes) = cjk_font();
    let font_ref = FontRef::from_index(&bytes, 0).expect("CJK face parses");
    let cmap = font_ref.charmap();
    let names = [
        "English", "Русский", "Español", "Deutsch", "Français", "Português",
        "中文", "日本語", "한국어", "العربية", "Italiano", "Türkçe", "Polski",
        "Українська", "हिन्दी",
    ];
    for name in names {
        for ch in name.chars() {
            let gid = cmap.map(ch as u32);
            for size in [10.5_f32, 11.25, 12.0, 12.5, 12.75, 13.0, 13.5, 14.25, 15.75, 16.5] {
                for subpx in 0..4u8 {
                    // A panic here is the bug; Err is graceful and fine.
                    let _ = rasterise_glyph(font, gid, size, subpx);
                }
            }
        }
    }
}

/// Sweep the CJK Unified Ideographs block (U+4E00..=U+9FFF) at one
/// fractional size — finds any single glyph whose charstring panics swash.
#[test]
#[ignore = "heavy: ~20k glyphs"]
fn cjk_ideograph_block_sweep() {
    let (font, bytes) = cjk_font();
    let font_ref = FontRef::from_index(&bytes, 0).expect("CJK face parses");
    let cmap = font_ref.charmap();
    let mut failures = 0u32;
    for cp in 0x4E00u32..=0x9FFF {
        let gid = cmap.map(cp);
        if gid == 0 { continue; }
        match rasterise_glyph(font, gid, 12.5, 1) {
            Ok(bm) => {
                assert_eq!(bm.alpha.len(), (bm.width * bm.height) as usize,
                    "buffer/placement mismatch at U+{cp:04X}");
            }
            Err(_) => failures += 1,
        }
    }
    eprintln!("ideograph sweep: {failures} graceful raster failures");
}
