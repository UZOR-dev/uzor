//! JS-authored animations for uzor with one API on web and native.
//!
//! A script is a canvas-2d-style function of time registered as
//! `globalThis.uzorAnims[name] = (ctx, t, p) => { ... }` (see
//! `js/prelude.js` for the `ctx` subset and `p` fields). Each frame it is
//! run, its recorded op stream is returned as an `f64` buffer and replayed
//! by [`uzor::render::replay_anim_ops`] onto any backend.
//!
//! - **web (wasm32)**: the page's own JS engine runs it. The prelude ships
//!   as a wasm-bindgen snippet (same-origin module, works under a CSP
//!   without `unsafe-eval`); scripts come from same-origin `<script>` tags or
//!   app snippets. [`JsAnimHost::load`] evaluates source text only where the
//!   CSP allows eval. No interpreter bytes in the wasm.
//! - **native**: embedded QuickJS behind the `quickjs` feature.
//!
//! ```ignore
//! let host = JsAnimHost::new()?;
//! host.load(include_str!("../anims/hexstar.js"))?;   // native; web: <script>
//! let mut anim = host.anim("hexstar");
//! anim.paint(ctx, t, &AnimParams { w: 64.0, h: 64.0, seed: 7, ..Default::default() }, &mut buf)?;
//! ```

pub use uzor::render::{replay_anim_ops, AnimParams, AnimRegistry, AnimScript, AnimScriptError, RenderContext};

/// The recorder prelude (ES module source), for hosts that load it themselves.
pub const PRELUDE_JS: &str = include_str!("../js/prelude.js");
/// Example script used by tests and demos.
pub const HEXSTAR_JS: &str = include_str!("../js/hexstar.js");

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::JsAnimHost;

#[cfg(all(not(target_arch = "wasm32"), feature = "quickjs"))]
mod native_qjs;
#[cfg(all(not(target_arch = "wasm32"), feature = "quickjs"))]
pub use native_qjs::JsAnimHost;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "quickjs")))]
mod native_none;
#[cfg(all(not(target_arch = "wasm32"), not(feature = "quickjs")))]
pub use native_none::JsAnimHost;

/// One registered animation of a [`JsAnimHost`].
pub struct JsAnim {
    host: JsAnimHost,
    name: String,
}

impl JsAnimHost {
    /// A host if this target has an engine, with `names` registered as the primary
    /// scripts in `reg` (the fallbacks already in `reg` paint while a script is missing).
    /// `None` (nothing registered) where no engine runs: native without `quickjs`.
    pub fn install(reg: &mut AnimRegistry, names: &[&str]) -> Option<Self> {
        let host = Self::new().ok()?;
        for n in names {
            reg.register(n, Box::new(host.anim(n)));
        }
        Some(host)
    }

    /// Handle for the animation registered under `name` (checked per frame,
    /// so a script may be loaded after the handle is made).
    pub fn anim(&self, name: &str) -> JsAnim {
        JsAnim { host: self.clone(), name: name.to_owned() }
    }
}

impl AnimScript for JsAnim {
    fn frame(&mut self, t: f64, p: &AnimParams, out: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        self.host.run(&self.name, t, &p.to_array(), out)
    }

    fn paint(
        &mut self,
        ctx: &mut dyn RenderContext,
        t: f64,
        p: &AnimParams,
        scratch: &mut Vec<f64>,
    ) -> Result<(), AnimScriptError> {
        self.host.run_and_replay(&self.name, t, &p.to_array(), ctx, scratch)
    }
}
