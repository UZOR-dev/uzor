//! Script-driven animations: a frame is a pure function of `(t, AnimParams)`
//! that emits a flat `f64` op stream ("anim ops"), which this module replays
//! onto any [`RenderContext`].
//!
//! The op stream is the contract between uzor and whatever produced it:
//! a JS function run by the page's engine (web), an embedded interpreter
//! (native, see the `uzor-anim-js` crate), a baked clip, or Rust code using
//! [`AnimOpWriter`]. Producers never touch a backend, so the same script
//! paints the same pixels on canvas2d, tiny-skia, vello or wgpu.
//!
//! JS authoring (engine + recorder live in `uzor-anim-js`): a script registers
//! `globalThis.uzorAnims[name] = (ctx, t, p) => { ... }` where `ctx` is a canvas-2d-shaped
//! recorder (paths, transforms, fill/stroke, `fillStyle`, `globalAlpha`, ...)
//! and `p = { w, h, seed, mouse: {x, y} | null, args: [a0..a7], rand }`.
//! `Math.random` is reseeded from `seed` every frame, so a frame is
//! deterministic in `(t, params)`.

use super::context::RenderContext;

/// Per-instance inputs of a script frame. Everything a frame may depend on
/// besides `t` — keeps frames deterministic and replayable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimParams {
    /// Logical size of the box the animation paints into (origin top-left).
    pub w: f64,
    pub h: f64,
    /// Per-instance seed (critter id, etc.); also seeds `Math.random`.
    pub seed: u32,
    /// Pointer in the same local coordinates, `None` when not hovering.
    pub mouse: Option<(f64, f64)>,
    /// Free numeric knobs (shape index, mood, intensity, ...).
    pub args: [f64; ANIM_ARGS],
}

/// Number of free knobs in [`AnimParams::args`].
pub const ANIM_ARGS: usize = 8;

impl Default for AnimParams {
    fn default() -> Self {
        Self { w: 64.0, h: 64.0, seed: 0, mouse: None, args: [0.0; ANIM_ARGS] }
    }
}

impl AnimParams {
    /// Wire layout shared with the JS runner `run(name, t, q)`:
    /// `[w, h, seed, mx, my, hover, a0, .., a7]`.
    pub fn to_array(&self) -> [f64; 6 + ANIM_ARGS] {
        let (mx, my, hv) = match self.mouse {
            Some((x, y)) => (x, y, 1.0),
            None => (0.0, 0.0, 0.0),
        };
        let mut q = [0.0; 6 + ANIM_ARGS];
        q[..6].copy_from_slice(&[self.w, self.h, self.seed as f64, mx, my, hv]);
        q[6..].copy_from_slice(&self.args);
        q
    }
}

/// Error from a script engine or from decoding its op stream.
#[derive(Clone, Debug, PartialEq)]
pub enum AnimScriptError {
    /// The engine is not available on this target/feature set.
    Unavailable(&'static str),
    /// The script (or its registration) failed; message from the engine.
    Script(String),
    /// The op stream was malformed at this index.
    BadOp(usize),
}

impl core::fmt::Display for AnimScriptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unavailable(why) => write!(f, "anim script engine unavailable: {why}"),
            Self::Script(m) => write!(f, "anim script error: {m}"),
            Self::BadOp(i) => write!(f, "anim op stream malformed at {i}"),
        }
    }
}

impl std::error::Error for AnimScriptError {}

/// Anything that turns `(t, params)` into an anim op stream.
pub trait AnimScript {
    /// Fill `out` (cleared first) with the ops of frame `t` (seconds).
    fn frame(&mut self, t: f64, params: &AnimParams, out: &mut Vec<f64>) -> Result<(), AnimScriptError>;

    /// Produce and paint frame `t`; `scratch` is reused across frames.
    fn paint(
        &mut self,
        ctx: &mut dyn RenderContext,
        t: f64,
        params: &AnimParams,
        scratch: &mut Vec<f64>,
    ) -> Result<(), AnimScriptError> {
        self.frame(t, params, scratch)?;
        replay_anim_ops(ctx, scratch)
    }
}

/// Op codes of the stream (kept in sync with `anim_script_prelude.js`).
pub mod op {
    pub const SAVE: u8 = 0;
    pub const RESTORE: u8 = 1;
    pub const TRANSLATE: u8 = 2;
    pub const ROTATE: u8 = 3;
    pub const SCALE: u8 = 4;
    pub const BEGIN_PATH: u8 = 5;
    pub const MOVE_TO: u8 = 6;
    pub const LINE_TO: u8 = 7;
    pub const QUAD_TO: u8 = 8;
    pub const BEZIER_TO: u8 = 9;
    pub const ARC: u8 = 10;
    pub const ELLIPSE: u8 = 11;
    pub const CLOSE_PATH: u8 = 12;
    pub const FILL: u8 = 13;
    pub const STROKE: u8 = 14;
    pub const FILL_COLOR: u8 = 15;
    pub const STROKE_COLOR: u8 = 16;
    pub const LINE_WIDTH: u8 = 17;
    pub const GLOBAL_ALPHA: u8 = 18;
    pub const FILL_RECT: u8 = 19;
    pub const STROKE_RECT: u8 = 20;
    pub const CLIP: u8 = 21;
    /// Canvas `transform(a, b, c, d, e, f)` — column `[a c e; b d f]`.
    pub const TRANSFORM: u8 = 22;
    /// `slot, sx, sy, sw, sh, dx, dy, dw, dh`. Negative dw/dh flips.
    pub const DRAW_IMAGE: u8 = 23;
    /// `slot` + source triangle (6) + dest triangle (6). One textured triangle.
    pub const DRAW_TRI: u8 = 24;
    /// Blur radius in px for the next image draw. `0` clears it.
    pub const FILTER: u8 = 25;

    /// Number of f64 operands following each op code.
    pub const fn arity(code: u8) -> Option<usize> {
        Some(match code {
            SAVE | RESTORE | BEGIN_PATH | CLOSE_PATH | FILL | STROKE | CLIP => 0,
            ROTATE | FILL_COLOR | STROKE_COLOR | LINE_WIDTH | GLOBAL_ALPHA | FILTER => 1,
            TRANSLATE | SCALE | MOVE_TO | LINE_TO => 2,
            QUAD_TO | FILL_RECT | STROKE_RECT => 4,
            ARC => 5,
            BEZIER_TO | TRANSFORM => 6,
            ELLIPSE => 7,
            DRAW_IMAGE => 9,
            DRAW_TRI => 13,
            _ => return None,
        })
    }
}

#[cfg(test)]
fn rgba_hex(c: u32) -> [u8; 9] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = [b'#'; 9];
    for i in 0..8 {
        s[1 + i] = HEX[((c >> (28 - 4 * i)) & 0xf) as usize];
    }
    s
}

/// Replay an anim op stream onto `ctx`. Unbalanced `save`s are closed at the
/// end so a script can never leak transform/clip state into the caller.
/// Image ops with no bound slot are skipped.
pub fn replay_anim_ops(ctx: &mut dyn RenderContext, ops: &[f64]) -> Result<(), AnimScriptError> {
    super::anim_bitmap::replay_anim_ops_bitmaps(ctx, ops, &[])
}

/// Rust-side producer of the same op stream (for ported/baked animations
/// and tests): `AnimOpWriter(&mut out).move_to(..)`.
pub struct AnimOpWriter<'a>(pub &'a mut Vec<f64>);

impl AnimOpWriter<'_> {
    fn push(&mut self, code: u8, args: &[f64]) {
        self.0.push(code as f64);
        self.0.extend_from_slice(args);
    }
    pub fn save(&mut self) { self.push(op::SAVE, &[]) }
    pub fn restore(&mut self) { self.push(op::RESTORE, &[]) }
    pub fn translate(&mut self, x: f64, y: f64) { self.push(op::TRANSLATE, &[x, y]) }
    pub fn rotate(&mut self, a: f64) { self.push(op::ROTATE, &[a]) }
    pub fn scale(&mut self, x: f64, y: f64) { self.push(op::SCALE, &[x, y]) }
    pub fn begin_path(&mut self) { self.push(op::BEGIN_PATH, &[]) }
    pub fn move_to(&mut self, x: f64, y: f64) { self.push(op::MOVE_TO, &[x, y]) }
    pub fn line_to(&mut self, x: f64, y: f64) { self.push(op::LINE_TO, &[x, y]) }
    pub fn close_path(&mut self) { self.push(op::CLOSE_PATH, &[]) }
    pub fn fill(&mut self) { self.push(op::FILL, &[]) }
    pub fn stroke(&mut self) { self.push(op::STROKE, &[]) }
    /// `rgba` packed as `0xRRGGBBAA`.
    pub fn fill_color(&mut self, rgba: u32) { self.push(op::FILL_COLOR, &[rgba as f64]) }
    pub fn stroke_color(&mut self, rgba: u32) { self.push(op::STROKE_COLOR, &[rgba as f64]) }
    pub fn line_width(&mut self, w: f64) { self.push(op::LINE_WIDTH, &[w]) }
    pub fn global_alpha(&mut self, a: f64) { self.push(op::GLOBAL_ALPHA, &[a]) }
}

/// A clip baked offline (e.g. by running the JS in node): `frames` op
/// streams sampled at `fps`, looped. No engine needed at runtime; params
/// other than `seed`-baked variants are ignored.
pub struct BakedAnim {
    pub fps: f64,
    pub frames: Vec<Vec<f64>>,
}

impl AnimScript for BakedAnim {
    fn frame(&mut self, t: f64, _p: &AnimParams, out: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        out.clear();
        if self.frames.is_empty() {
            return Ok(());
        }
        let n = self.frames.len() as i64;
        let i = ((t * self.fps).floor() as i64).rem_euclid(n) as usize;
        out.extend_from_slice(&self.frames[i]);
        Ok(())
    }
}

/// A Rust painter as an [`AnimScript`]: draws straight onto the context (no op stream).
/// The usual fallback for a JS animation where no engine runs (native without
/// `uzor-anim-js/quickjs`), or the final form of a ported one. [`AnimScript::frame`]
/// reports [`AnimScriptError::Unavailable`]: it has nothing to record.
pub struct FnAnim<F>(pub F);

impl<F: FnMut(&mut dyn RenderContext, f64, &AnimParams)> AnimScript for FnAnim<F> {
    fn frame(&mut self, _t: f64, _p: &AnimParams, out: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        out.clear();
        Err(AnimScriptError::Unavailable("FnAnim paints directly"))
    }
    fn paint(&mut self, ctx: &mut dyn RenderContext, t: f64, p: &AnimParams, _s: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        (self.0)(ctx, t, p);
        Ok(())
    }
}

/// Which producer painted an animation's frame ([`AnimRegistry::paint`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimSource {
    /// The registered script (a JS function, a baked clip, ...).
    Primary,
    /// The fallback painter: no primary, or it failed.
    Fallback,
}

struct AnimEntry {
    name: String,
    primary: Option<Box<dyn AnimScript>>,
    fallback: Option<Box<dyn AnimScript>>,
    /// The primary failed: skipped until this time (a script loaded later gets retried).
    retry_at: f64,
    last_error: Option<AnimScriptError>,
}

/// Animations by name, as the DOM's animation nodes
/// ([`El::anim`](crate::ui::dom::El::anim)) find them: a primary script per name (usually
/// a JS function through `uzor-anim-js`) and a fallback painter (Rust or [`BakedAnim`])
/// used while the primary is missing or failing. An engine that is not there
/// ([`AnimScriptError::Unavailable`]) is never retried; a script error is retried after
/// a second (its `<script>` may still be loading).
#[derive(Default)]
pub struct AnimRegistry {
    entries: Vec<AnimEntry>,
    scratch: Vec<f64>,
}

impl AnimRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn entry(&mut self, name: &str) -> &mut AnimEntry {
        let i = match self.entries.iter().position(|e| e.name == name) {
            Some(i) => i,
            None => {
                self.entries.push(AnimEntry { name: name.to_owned(), primary: None, fallback: None, retry_at: f64::NEG_INFINITY, last_error: None });
                self.entries.len() - 1
            }
        };
        &mut self.entries[i]
    }

    /// Set the primary script of `name` (replaces an earlier one).
    pub fn register(&mut self, name: &str, script: Box<dyn AnimScript>) -> &mut Self {
        let e = self.entry(name);
        e.primary = Some(script);
        e.retry_at = f64::NEG_INFINITY;
        e.last_error = None;
        self
    }

    /// Set the fallback painter of `name`.
    pub fn fallback(&mut self, name: &str, script: Box<dyn AnimScript>) -> &mut Self {
        self.entry(name).fallback = Some(script);
        self
    }

    /// Something can paint `name`.
    pub fn has(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name && (e.primary.is_some() || e.fallback.is_some()))
    }

    /// The last error of `name`'s primary, if it failed.
    pub fn last_error(&self, name: &str) -> Option<&AnimScriptError> {
        self.entries.iter().find(|e| e.name == name).and_then(|e| e.last_error.as_ref())
    }

    /// Paint frame `t` of `name` with `params` at the context's origin (the caller
    /// translates and clips). Tries the primary, then the fallback.
    pub fn paint(&mut self, ctx: &mut dyn RenderContext, name: &str, t: f64, params: &AnimParams) -> Result<AnimSource, AnimScriptError> {
        let Some(e) = self.entries.iter_mut().find(|e| e.name == name) else {
            return Err(AnimScriptError::Unavailable("no animation registered under this name"));
        };
        let scratch = &mut self.scratch;
        if let Some(p) = e.primary.as_mut().filter(|_| t >= e.retry_at) {
            // Record first, so a failing script paints nothing half-way.
            match p.frame(t, params, scratch) {
                Ok(()) => {
                    e.last_error = None;
                    replay_anim_ops(ctx, scratch)?;
                    return Ok(AnimSource::Primary);
                }
                Err(err) => {
                    e.retry_at = if matches!(err, AnimScriptError::Unavailable(_)) { f64::INFINITY } else { t + 1.0 };
                    e.last_error = Some(err);
                }
            }
        }
        match e.fallback.as_mut() {
            Some(f) => f.paint(ctx, t, params, scratch).map(|()| AnimSource::Fallback),
            None => Err(e.last_error.clone().unwrap_or(AnimScriptError::Unavailable("no fallback painter"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_hex_roundtrip() {
        assert_eq!(&rgba_hex(0xf2a93bff), b"#f2a93bff");
        assert_eq!(crate::render::parse_color(core::str::from_utf8(&rgba_hex(0x11223344)).unwrap()), (0x11, 0x22, 0x33, 0x44));
    }

    #[test]
    fn arity_covers_all_codes() {
        for c in 0..=25u8 {
            assert!(op::arity(c).is_some(), "code {c}");
        }
        assert!(op::arity(26).is_none());
    }

    #[test]
    fn params_wire_layout() {
        let p = AnimParams { w: 10.0, h: 20.0, seed: 3, mouse: Some((1.0, 2.0)), args: [4.0, 5.0, 6.0, 7.0, 0.0, 0.0, 0.0, 8.0] };
        assert_eq!(p.to_array(), [10.0, 20.0, 3.0, 1.0, 2.0, 1.0, 4.0, 5.0, 6.0, 7.0, 0.0, 0.0, 0.0, 8.0]);
    }

    struct Fails(AnimScriptError, std::rc::Rc<std::cell::Cell<u32>>);
    impl AnimScript for Fails {
        fn frame(&mut self, _t: f64, _p: &AnimParams, _o: &mut Vec<f64>) -> Result<(), AnimScriptError> {
            self.1.set(self.1.get() + 1);
            Err(self.0.clone())
        }
    }

    #[test]
    fn registry_falls_back_and_retries_script_errors_only() {
        use crate::testing::RecordingRenderContext;
        let mut ctx = RecordingRenderContext::new();
        let p = AnimParams::default();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut reg = AnimRegistry::new();
        assert!(reg.paint(&mut ctx, "x", 0.0, &p).is_err());
        reg.register("x", Box::new(Fails(AnimScriptError::Script("not loaded".into()), calls.clone())));
        let hits = std::rc::Rc::new(std::cell::Cell::new(0));
        let h2 = hits.clone();
        reg.fallback("x", Box::new(FnAnim(move |_: &mut dyn RenderContext, _t: f64, _p: &AnimParams| h2.set(h2.get() + 1))));
        assert_eq!(reg.paint(&mut ctx, "x", 0.0, &p), Ok(AnimSource::Fallback));
        assert_eq!(reg.paint(&mut ctx, "x", 0.5, &p), Ok(AnimSource::Fallback));
        assert_eq!(calls.get(), 1, "a script error waits a second before the next try");
        reg.paint(&mut ctx, "x", 1.0, &p).unwrap();
        assert_eq!((calls.get(), hits.get()), (2, 3));
        assert!(matches!(reg.last_error("x"), Some(AnimScriptError::Script(_))));
        // A baked primary takes over and paints from its op stream.
        let mut ops = Vec::new();
        AnimOpWriter(&mut ops).fill_color(0xff0000ff);
        reg.register("x", Box::new(BakedAnim { fps: 1.0, frames: vec![ops] }));
        assert_eq!(reg.paint(&mut ctx, "x", 2.0, &p), Ok(AnimSource::Primary));
        assert!(reg.last_error("x").is_none());
        // No engine: never retried.
        let c2 = std::rc::Rc::new(std::cell::Cell::new(0));
        reg.register("y", Box::new(Fails(AnimScriptError::Unavailable("none"), c2.clone())));
        reg.fallback("y", Box::new(FnAnim(|_: &mut dyn RenderContext, _t: f64, _p: &AnimParams| {})));
        for t in 0..5 {
            assert_eq!(reg.paint(&mut ctx, "y", t as f64 * 10.0, &p), Ok(AnimSource::Fallback));
        }
        assert_eq!(c2.get(), 1);
    }
}
