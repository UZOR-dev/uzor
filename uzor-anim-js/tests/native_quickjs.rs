//! Runs the JS hexstar prototype through embedded QuickJS and paints it
//! with the tiny-skia backend. `UZOR_ANIM_OUT=dir` also writes PNG frames.
#![cfg(feature = "quickjs")]

use uzor_anim_js::{AnimParams, AnimScript, JsAnimHost, HEXSTAR_JS};
use uzor_render_tiny_skia::TinySkiaCpuRenderContext;

fn params() -> AnimParams {
    AnimParams { w: 128.0, h: 128.0, seed: 7, ..Default::default() }
}

#[test]
fn hexstar_is_deterministic_and_paints() {
    let host = JsAnimHost::new().unwrap();
    host.load(HEXSTAR_JS).unwrap();
    let mut anim = host.anim("hexstar");
    let (mut a, mut b) = (Vec::new(), Vec::new());
    anim.frame(1.25, &params(), &mut a).unwrap();
    anim.frame(1.25, &params(), &mut b).unwrap();
    assert_eq!(a, b, "same (t, params) must give the same ops");
    anim.frame(1.30, &params(), &mut b).unwrap();
    assert_ne!(a, b, "the animation must move");

    let out = std::env::var_os("UZOR_ANIM_OUT");
    let mut scratch = Vec::new();
    for i in 0..8 {
        let mut ctx = TinySkiaCpuRenderContext::new(128, 128, 1.0);
        ctx.clear(tiny_skia::Color::from_rgba8(14, 18, 26, 255));
        anim.paint(&mut ctx, i as f64 * 0.35, &params(), &mut scratch).unwrap();
        let lit = ctx.pixels().chunks_exact(4).filter(|p| p[0] > 200 && p[1] > 150).count();
        assert!(lit > 200, "frame {i}: star/hex outline should be visible ({lit} px)");
        if let Some(dir) = &out {
            let p = std::path::Path::new(dir).join(format!("hexstar-{i}.png"));
            ctx.pixmap().save_png(p).unwrap();
        }
    }
}

#[test]
fn script_errors_surface() {
    let host = JsAnimHost::new().unwrap();
    let mut missing = host.anim("nope");
    let e = missing.frame(0.0, &params(), &mut Vec::new()).unwrap_err();
    assert!(e.to_string().contains("not registered"), "{e}");
    assert!(host.load("throw new Error('boom')").unwrap_err().to_string().contains("boom"));
}

/// Web parity: node (V8, i.e. what the page runs) must emit the very same
/// op stream as embedded QuickJS. Skipped when node is not installed.
#[test]
fn matches_v8_when_node_available() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let js = format!(
        "import('{dir}/js/prelude.js').then(async m=>{{await import('{dir}/js/hexstar.js');\
         process.stdout.write(JSON.stringify(Array.from(m.uzor_anim_run('hexstar',3.7,[128,128,7,40,50,1,0,0,0,0]))))}})"
    );
    let Ok(o) = std::process::Command::new("node").args(["--input-type=module", "-e", &js]).output() else {
        eprintln!("node not found; skipping");
        return;
    };
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v8: Vec<f64> = String::from_utf8(o.stdout).unwrap().trim_matches(|c| c == '[' || c == ']')
        .split(',').map(|s| s.parse().unwrap()).collect();
    let host = JsAnimHost::new().unwrap();
    host.load(HEXSTAR_JS).unwrap();
    let mut q = Vec::new();
    let p = AnimParams { mouse: Some((40.0, 50.0)), ..params() };
    host.anim("hexstar").frame(3.7, &p, &mut q).unwrap();
    assert_eq!(q.len(), v8.len());
    let max = q.iter().zip(&v8).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    assert!(max < 1e-9, "quickjs vs v8 max diff {max}");
}

/// A script registered by name runs as the primary of a DOM animation node: laid out by
/// flex, clipped to its box, with its args, and it asks for the next frame.
#[test]
fn script_runs_in_a_dom_node() {
    use uzor::ui::dom::{lay_out, paint_with, AnimFrame, AnimNode, El};
    use uzor::ui::style::{Sheet, State};
    let mut reg = uzor_anim_js::AnimRegistry::new();
    reg.fallback("hexstar", Box::new(uzor::render::FnAnim(|_: &mut dyn uzor::render::RenderContext, _: f64, _: &AnimParams| {})));
    let host = JsAnimHost::install(&mut reg, &["hexstar"]).unwrap();
    host.load(HEXSTAR_JS).unwrap();
    assert!(host.has("hexstar") && !host.has("nope"));
    let sh = Sheet::parse("div.row { display: flex; padding: 8px } div.av { width: 48px; height: 48px }").unwrap();
    let root = El::new("div.row").child(El::new("div.av").id("av").anim(AnimNode::new("hexstar").seed(7).args([1.0])));
    let mut ctx = TinySkiaCpuRenderContext::new(64, 64, 1.0);
    let d = lay_out(&mut ctx, &sh, &root, 0.0, 0.0, 64.0, &|_| State::NONE);
    let r = paint_with(&mut ctx, &d, &mut reg, &AnimFrame::new(1.0, uzor::Rect::new(0.0, 0.0, 64.0, 64.0)));
    assert_eq!((r.painted, r.next), (1, Some(1.0 / 30.0)));
    assert!(reg.last_error("hexstar").is_none(), "the JS primary painted");
    let px = ctx.pixels();
    let inside = |x: usize, y: usize| px[(y * 64 + x) * 4 + 3] > 0;
    assert!((0..64).flat_map(|y| (0..64).map(move |x| (x, y))).filter(|&(x, y)| inside(x, y)).count() > 200, "painted");
    assert!((0..64).all(|i| !inside(i, 2) && !inside(2, i) && !inside(i, 61) && !inside(61, i)), "clipped to the 48px box");
}
