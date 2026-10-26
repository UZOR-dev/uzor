use js_sys::{Float64Array, Reflect};
use wasm_bindgen::prelude::*;

use uzor::render::{replay_anim_ops, RenderContext};

use crate::AnimScriptError;

// The prelude is a same-origin wasm-bindgen snippet: imported by the glue at
// init, so it works under `script-src 'self'` without `unsafe-eval`.
#[wasm_bindgen(module = "/js/prelude.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn uzor_anim_run(name: &str, t: f64, q: &[f64]) -> Result<Float64Array, JsValue>;
    fn uzor_anim_has(name: &str) -> bool;
}

/// Web host: the page's JS engine. Zero interpreter bytes in the wasm.
#[derive(Clone)]
pub struct JsAnimHost(());

fn js_err(e: JsValue) -> AnimScriptError {
    let msg = Reflect::get(&e, &"message".into())
        .ok()
        .and_then(|m| m.as_string())
        .or_else(|| e.as_string())
        .unwrap_or_else(|| "js exception".into());
    AnimScriptError::Script(msg)
}

impl JsAnimHost {
    pub fn new() -> Result<Self, AnimScriptError> {
        Ok(Self(()))
    }

    /// Evaluate script source. Needs a CSP that allows eval; under a strict
    /// CSP serve the script as a same-origin `<script src>` instead (it
    /// registers itself into `globalThis.uzorAnims`) and skip `load`.
    pub fn load(&self, src: &str) -> Result<(), AnimScriptError> {
        js_sys::eval(src).map(|_| ()).map_err(js_err)
    }

    /// A script registered `name` (`globalThis.uzorAnims[name]`).
    pub fn has(&self, name: &str) -> bool {
        uzor_anim_has(name)
    }

    pub(crate) fn run(&self, name: &str, t: f64, q: &[f64; 14], out: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        let a = uzor_anim_run(name, t, q).map_err(js_err)?;
        out.clear();
        out.resize(a.length() as usize, 0.0);
        a.copy_to(out);
        Ok(())
    }

    pub(crate) fn run_and_replay(
        &self,
        name: &str,
        t: f64,
        q: &[f64; 14],
        ctx: &mut dyn RenderContext,
        scratch: &mut Vec<f64>,
    ) -> Result<(), AnimScriptError> {
        self.run(name, t, q, scratch)?;
        replay_anim_ops(ctx, scratch)
    }
}
