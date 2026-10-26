use crate::AnimScriptError;

/// Stub host: built without the `quickjs` feature, every call reports
/// [`AnimScriptError::Unavailable`] so callers can fall back (static icon,
/// baked clip) with the same code path.
#[derive(Clone)]
pub struct JsAnimHost(());

const WHY: &str = "uzor-anim-js built without the `quickjs` feature";

impl JsAnimHost {
    pub fn new() -> Result<Self, AnimScriptError> {
        Err(AnimScriptError::Unavailable(WHY))
    }
    pub fn load(&self, _src: &str) -> Result<(), AnimScriptError> {
        Err(AnimScriptError::Unavailable(WHY))
    }
    pub fn has(&self, _name: &str) -> bool {
        false
    }
    pub(crate) fn run(&self, _n: &str, _t: f64, _q: &[f64; 14], _o: &mut Vec<f64>) -> Result<(), AnimScriptError> {
        Err(AnimScriptError::Unavailable(WHY))
    }
    pub(crate) fn run_and_replay(
        &self,
        name: &str,
        t: f64,
        q: &[f64; 14],
        _ctx: &mut dyn uzor::render::RenderContext,
        out: &mut Vec<f64>,
    ) -> Result<(), AnimScriptError> {
        self.run(name, t, q, out)
    }
}
