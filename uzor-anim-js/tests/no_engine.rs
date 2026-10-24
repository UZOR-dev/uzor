//! Native without the `quickjs` feature: no host, the registry's fallback paints.
#![cfg(all(not(target_arch = "wasm32"), not(feature = "quickjs")))]

use uzor::render::{AnimParams, AnimRegistry, AnimSource, FnAnim, RenderContext};
use uzor_anim_js::JsAnimHost;

#[test]
fn install_without_engine_leaves_the_fallback() {
    let mut reg = AnimRegistry::new();
    reg.fallback("spin", Box::new(FnAnim(|c: &mut dyn RenderContext, _t: f64, _p: &AnimParams| c.fill_rect(0.0, 0.0, 1.0, 1.0))));
    assert!(JsAnimHost::install(&mut reg, &["spin"]).is_none());
    let mut ctx = uzor_render_tiny_skia::TinySkiaCpuRenderContext::new(4, 4, 1.0);
    assert_eq!(reg.paint(&mut ctx, "spin", 0.0, &AnimParams::default()), Ok(AnimSource::Fallback));
}
