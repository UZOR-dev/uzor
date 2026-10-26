use std::cell::RefCell;
use std::rc::Rc;

use rquickjs::{Context, Function, Module, Object, Runtime, TypedArray};
use rquickjs_jit::{Jit, JitConfig};

use uzor::render::{replay_anim_ops_bitmaps, AnimBitmap, RenderContext};

use crate::{AnimScriptError, PRELUDE_JS};

/// Native host: one QuickJS runtime/context, shared by its `JsAnim`s
/// (single-threaded, like the UI thread that paints them).
#[derive(Clone)]
pub struct JsAnimHost(Rc<Inner>);

struct Inner {
    // Fields drop in declaration order. Detach the JIT before the runtime.
    _jit: Jit,
    _rt: Runtime,
    ctx: Context,
    slots: RefCell<Vec<AnimBitmap>>,
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
        // Rider + palm + a few offscreen copies of the rider. 32 MiB is short of that.
        rt.set_memory_limit(96 << 20);
        let jit = Jit::attach(&rt, JitConfig::default())
            .map_err(|e| AnimScriptError::Script(format!("quickjs-jit attach: {e:?}")))?;
        jit.require_native()
            .map_err(|e| AnimScriptError::Script(format!("quickjs-jit native: {e:?}")))?;
        let ctx = Context::full(&rt).map_err(|e| AnimScriptError::Script(e.to_string()))?;
        ctx.with(|c| -> Result<(), AnimScriptError> {
            let m = Module::declare(c.clone(), "uzor-anim-prelude", PRELUDE_JS).map_err(|e| err(e, &c))?;
            let (_m, p) = m.eval().map_err(|e| err(e, &c))?;
            p.finish::<()>().map_err(|e| err(e, &c))?;
            Ok(())
        })?;
        Ok(Self(Rc::new(Inner {
            _jit: jit,
            _rt: rt,
            ctx,
            slots: RefCell::new(Vec::new()),
        })))
    }

    /// Evaluate script source (classic script) that registers animations.
    pub fn load(&self, src: &str) -> Result<(), AnimScriptError> {
        self.0.ctx.with(|c| c.eval::<(), _>(src).map_err(|e| err(e, &c)))
    }

    /// Install `rgba` as `globalThis[name]` and as bitmap `slot`.
    /// `_dirty: false` so the script does not copy the bytes back through JS.
    pub fn bind_bitmap(
        &self,
        name: &str,
        rgba: &[u8],
        w: u32,
        h: u32,
        slot: u32,
        toon: bool,
    ) -> Result<(), AnimScriptError> {
        let need = w as usize * h as usize * 4;
        if w == 0 || h == 0 || rgba.len() != need {
            return Err(AnimScriptError::Script(format!("bitmap {name} size")));
        }
        {
            let mut slots = self.0.slots.borrow_mut();
            let idx = slot as usize;
            if slots.len() <= idx {
                slots.resize_with(idx + 1, AnimBitmap::default);
            }
            slots[idx] = AnimBitmap {
                w,
                h,
                rgba: rgba.to_vec(),
                gen: 1,
            };
        }
        self.0.ctx.with(|c| {
            let bytes = TypedArray::<u8>::new(c.clone(), rgba.to_vec()).map_err(|e| err(e, &c))?;
            let obj = Object::new(c.clone()).map_err(|e| err(e, &c))?;
            obj.set("width", w).map_err(|e| err(e, &c))?;
            obj.set("height", h).map_err(|e| err(e, &c))?;
            obj.set("naturalWidth", w).map_err(|e| err(e, &c))?;
            obj.set("naturalHeight", h).map_err(|e| err(e, &c))?;
            obj.set("_w", w).map_err(|e| err(e, &c))?;
            obj.set("_h", h).map_err(|e| err(e, &c))?;
            obj.set("complete", true).map_err(|e| err(e, &c))?;
            obj.set("_toon", toon).map_err(|e| err(e, &c))?;
            obj.set("_slot", slot).map_err(|e| err(e, &c))?;
            obj.set("_dirty", false).map_err(|e| err(e, &c))?;
            obj.set("_rgba", bytes).map_err(|e| err(e, &c))?;
            c.globals().set(name, obj).map_err(|e| err(e, &c))?;
            Ok(())
        })
    }

    /// A script registered `name` (`globalThis.uzorAnims[name]`).
    pub fn has(&self, name: &str) -> bool {
        self.0.ctx.with(|c| {
            let g: Option<rquickjs::Object> = c.globals().get("uzorAnims").ok();
            g.is_some_and(|o| o.contains_key(name).unwrap_or(false))
        })
    }

    pub(crate) fn run(&self, name: &str, t: f64, q: &[f64; 14], out: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        // Install code compiled on the previous frame. poll stays on this thread.
        self.0._jit.poll();
        self.0.ctx.with(|c| {
            let f: Function = c.globals().get("__uzorAnimRun").map_err(|e| err(e, &c))?;
            let a: TypedArray<f64> = f.call((name, t, q.to_vec())).map_err(|e| err(e, &c))?;
            // SAFETY: copied before the next call into the engine. JS is not run while the slice is live.
            let bytes = unsafe { a.as_bytes() }.ok_or_else(|| AnimScriptError::Script("detached op buffer".into()))?;
            out.clear();
            out.extend(bytes.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().unwrap())));
            Ok(())
        })
    }

    fn drain_publish(&self) -> Result<(), AnimScriptError> {
        self.0.ctx.with(|c| {
            let arr: rquickjs::Array = match c.globals().get("__uzorPublish") {
                Ok(a) => a,
                Err(_) => return Ok(()),
            };
            let len = arr.len();
            let mut slots = self.0.slots.borrow_mut();
            for i in 0..len {
                let item: Object = match arr.get(i) {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                let Some(id) = js_f64(&item, "id") else { continue };
                let Some(w) = js_f64(&item, "w") else { continue };
                let Some(h) = js_f64(&item, "h") else { continue };
                let Ok(px) = item.get::<_, TypedArray<u8>>("px") else { continue };
                if id < 0.0 || id.fract() != 0.0 {
                    continue;
                }
                let id = id as usize;
                let w = w as u32;
                let h = h as u32;
                let need = w as usize * h as usize * 4;
                // SAFETY: copied before the next call into the engine.
                let rgba = {
                    let Some(bytes) = (unsafe { px.as_bytes() }) else { continue };
                    if w == 0 || h == 0 || bytes.len() < need {
                        continue;
                    }
                    bytes[..need].to_vec()
                };
                if slots.len() <= id {
                    slots.resize_with(id + 1, AnimBitmap::default);
                }
                let gen = slots[id].gen.wrapping_add(1);
                slots[id] = AnimBitmap {
                    w,
                    h,
                    rgba,
                    gen,
                };
            }
            Ok(())
        })
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
        self.drain_publish()?;
        let slots = self.0.slots.borrow();
        replay_anim_ops_bitmaps(ctx, scratch, &slots)
    }
}

fn js_f64(obj: &Object<'_>, key: &str) -> Option<f64> {
    let v: rquickjs::Value = obj.get(key).ok()?;
    v.as_int().map(|i| i as f64).or_else(|| v.as_float())
}
