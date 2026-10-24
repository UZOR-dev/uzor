use std::rc::Rc;

use rquickjs::{Context, Function, Module, Runtime, TypedArray};

use crate::{AnimScriptError, PRELUDE_JS};

/// Native host: one QuickJS runtime/context, shared by its `JsAnim`s
/// (single-threaded, like the UI thread that paints them).
#[derive(Clone)]
pub struct JsAnimHost(Rc<Inner>);

struct Inner {
    _rt: Runtime,
    ctx: Context,
}

fn err(e: rquickjs::Error, ctx: &rquickjs::Ctx<'_>) -> AnimScriptError {
    if let rquickjs::Error::Exception = e {
        let ex = ctx.catch();
        let msg = ex
            .as_exception()
            .and_then(|x| x.message())
            .or_else(|| ex.as_string().and_then(|s| s.to_string().ok()))
            .unwrap_or_else(|| format!("{ex:?}"));
        return AnimScriptError::Script(msg);
    }
    AnimScriptError::Script(e.to_string())
}

impl JsAnimHost {
    pub fn new() -> Result<Self, AnimScriptError> {
        let rt = Runtime::new().map_err(|e| AnimScriptError::Script(e.to_string()))?;
        // Animations are tiny; cap the heap so a runaway script can't eat RAM.
        rt.set_memory_limit(32 << 20);
        let ctx = Context::full(&rt).map_err(|e| AnimScriptError::Script(e.to_string()))?;
        ctx.with(|c| -> Result<(), AnimScriptError> {
            let m = Module::declare(c.clone(), "uzor-anim-prelude", PRELUDE_JS).map_err(|e| err(e, &c))?;
            let (_m, p) = m.eval().map_err(|e| err(e, &c))?;
            p.finish::<()>().map_err(|e| err(e, &c))?;
            Ok(())
        })?;
        Ok(Self(Rc::new(Inner { _rt: rt, ctx })))
    }

    /// Evaluate script source (classic script) that registers animations.
    pub fn load(&self, src: &str) -> Result<(), AnimScriptError> {
        self.0.ctx.with(|c| c.eval::<(), _>(src).map_err(|e| err(e, &c)))
    }

    /// A script registered `name` (`globalThis.uzorAnims[name]`).
    pub fn has(&self, name: &str) -> bool {
        self.0.ctx.with(|c| {
            let g: Option<rquickjs::Object> = c.globals().get("uzorAnims").ok();
            g.is_some_and(|o| o.contains_key(name).unwrap_or(false))
        })
    }

    pub(crate) fn run(&self, name: &str, t: f64, q: &[f64; 14], out: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        self.0.ctx.with(|c| {
            let f: Function = c.globals().get("__uzorAnimRun").map_err(|e| err(e, &c))?;
            let a: TypedArray<f64> = f.call((name, t, q.to_vec())).map_err(|e| err(e, &c))?;
            let bytes = a.as_bytes().ok_or_else(|| AnimScriptError::Script("detached op buffer".into()))?;
            out.clear();
            out.extend(bytes.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().unwrap())));
            Ok(())
        })
    }
}
