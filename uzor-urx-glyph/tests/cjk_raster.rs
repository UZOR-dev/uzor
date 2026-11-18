//! CJK glyph rasterisation — the us General tab language names
//! (中文 / 日本語 / 한국어) must rasterise through the same swash path
//! every other face uses, never panic. The font bytes come from
//! `uzor-fonts`' opt-in `cjk` feature (Noto Sans CJK SC, CFF/`OTTO`),
//! per the owner directive that fonts are sourced from uzor-fonts.
//!
//! Regression for the CJK panic seen when the URX text path first met
//! a real CJK run: `rasterise_glyph` must either produce a bitmap or
//! return a `GlyphError`, and `draw_glyph_run` must keep compositing
//! the rest of the run when one glyph fails.

use swash::FontRef;
use uzor_urx_core::scene::Glyph;
use uzor_urx_glyph::{draw_glyph_run, rasterise_glyph, register_font, GlyphError};

/// The language names the us General tab lists (mlc user settings).
const CJK_TEXT: &str = "中文日本語한국어";

fn cjk_font() -> (uzor_urx_core::scene::FontId, Vec<u8>) {
    let bytes = uzor_fonts::NOTO_SANS_CJK_SC.to_vec();
    assert_eq!(&bytes[..4], b"OTTO", "the cjk feature must provide the real CFF face");
    let id = register_font(bytes.clone()).expect("register CJK face");
    (id, bytes)
}

fn glyph_id_for(bytes: &[u8], ch: char) -> u16 {
    let font = FontRef::from_index(bytes, 0).expect("CJK face parses");
    font.charmap().map(ch as u32)
}

#[test]
fn cjk_glyphs_rasterise_without_panic() {
    let (font, bytes) = cjk_font();
    for ch in CJK_TEXT.chars() {
        let gid = glyph_id_for(&bytes, ch);
        assert_ne!(gid, 0, "the CJK face covers {ch}");
        for size in [11.0_f32, 13.0, 16.0, 24.0] {
            for subpx in 0..4u8 {
                match rasterise_glyph(font, gid, size, subpx) {
                    Ok(bm) => {
                        assert_eq!(
                            bm.alpha.len(),
                            (bm.width * bm.height) as usize,
                            "bitmap buffer matches placement for {ch} at {size}px"
                        );
                    }
                    Err(GlyphError::RasterFailed) => {
                        // Graceful failure is acceptable; a panic is not.
                    }
                    Err(e) => panic!("unexpected glyph error for {ch}: {e:?}"),
                }
            }
        }
    }
}

#[test]
fn cjk_glyph_run_composites() {
    let (font, bytes) = cjk_font();
    let glyphs: Vec<Glyph> = CJK_TEXT
        .chars()
        .enumerate()
        .map(|(i, ch)| Glyph { glyph_id: glyph_id_for(&bytes, ch) as u32, x: i as f32 * 16.0, y: 0.0 })
        .collect();
    let mut buf = vec![0u8; 256 * 48 * 4];
    draw_glyph_run(&mut buf, 256, 48, 8.0, 32.0, &glyphs, font, 16.0, [230, 230, 235, 255], None, None)
        .expect("CJK run composites");
    let painted = buf.chunks_exact(4).filter(|c| c[3] > 0).count();
    assert!(painted > 200, "CJK run must leave ink, got {painted} px");
}

/// The exact us-language-list panic trigger: cosmic-text marks a glyph
/// it could not place with pen x/y = f32::MAX (unsupported-script
/// fallback). The compositor's old `(px.floor() as i32) + bm.left`
/// overflowed i32 in debug. Sentinel pens must clip away like any
/// off-canvas draw while the placeable glyphs of the run still paint.
#[test]
fn unplaceable_glyph_sentinel_pen_does_not_panic() {
    let (font, bytes) = cjk_font();
    let gid = glyph_id_for(&bytes, '中') as u32;
    let glyphs = vec![
        Glyph { glyph_id: gid, x: f32::MAX, y: 0.0 },
        Glyph { glyph_id: gid, x: 8.0, y: 0.0 },
        Glyph { glyph_id: gid, x: 0.0, y: f32::MAX },
        Glyph { glyph_id: gid, x: f32::NEG_INFINITY, y: 0.0 },
    ];
    let mut buf = vec![0u8; 64 * 32 * 4];
    draw_glyph_run(&mut buf, 64, 32, 4.0, 24.0, &glyphs, font, 13.0, [255, 255, 255, 255], None, None)
        .expect("sentinel pens are clipped, not fatal");
    let painted = buf.chunks_exact(4).filter(|c| c[3] > 0).count();
    assert!(painted > 20, "the placeable glyph still painted, got {painted} px");
}

#[test]
fn missing_glyph_ids_fail_gracefully_not_panic() {
    let (font, _bytes) = cjk_font();
    // Glyph ids beyond the face's glyph count must not panic the
    // rasteriser — a shaped run can carry a stale/foreign id after a
    // font fallback boundary; the run must keep going.
    for gid in [0u16, 65535] {
        let _ = rasterise_glyph(font, gid, 13.0, 0);
    }
    let glyphs = vec![
        Glyph { glyph_id: 65535, x: 0.0, y: 0.0 },
        Glyph { glyph_id: glyph_id_for(&_bytes, '中') as u32, x: 16.0, y: 0.0 },
    ];
    let mut buf = vec![0u8; 64 * 32 * 4];
    // One bad glyph must not abort the run: the valid glyph after it
    // still composites (graceful missing-glyph handling).
    draw_glyph_run(&mut buf, 64, 32, 4.0, 24.0, &glyphs, font, 13.0, [255, 255, 255, 255], None, None)
        .expect("a run with one bad glyph still composites the rest");
    let painted = buf.chunks_exact(4).filter(|c| c[3] > 0).count();
    assert!(painted > 20, "the valid glyph after the bad one painted, got {painted} px");
}
