//! Renderer prewarm for the native host.
//!
//! `run_native` starts [`Startup`] before it builds the event loop: a thread
//! that probes the adapter (the hub) and builds the device and the active
//! backend's renderer with all its pipelines ([`uzor_render_hub::prewarm`]),
//! through a pipeline cache kept on disk, then draws a small off-screen scene
//! with it ([`PrewarmConfig::warm_up_draw`]) so drivers that compile shader
//! variants on first use (and the surface blit) pay that before a window
//! exists. The event loop, the runtime and the app's first tick run meanwhile;
//! the first window binds the result to its surface, so its first frame does
//! no device, pipeline or first-use compile work. Later windows
//! and Android's surfaces on resume prewarm on the spot (same cache).
//!
//! Where the platform keeps what was presented to a hidden window
//! ([`PrewarmConfig::hold_until_first_frame`], on by default on Windows) the
//! window stays hidden until its first frame is presented. Elsewhere it is
//! shown right before that frame: an unmapped X11 window drops what is drawn
//! into it, a hidden macOS window's Metal layer withholds drawables (up to a
//! second), and Wayland ignores visibility. A held window gets one more frame
//! once it is shown. `UZOR_TRACE_STARTUP=1` prints the startup stages.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::Instant;

use uzor::RenderBackend;
use uzor_render_hub::{PipelineCacheStore, PrewarmOptions, Prewarmed, RenderHub};

/// The native host's prewarm switches ([`super::NativeOptions::prewarm`]).
#[derive(Clone, Debug)]
pub struct PrewarmConfig {
    /// Build the device and the renderer before the window exists (on by
    /// default). `UZOR_PREWARM=0` turns it off at run time.
    pub enabled: bool,
    /// Keep compiled pipelines on disk between runs (`wgpu::PipelineCache`,
    /// Vulkan only) under the platform cache directory, `uzor/<app>/`.
    pub pipeline_cache: bool,
    /// The `<app>` directory name; `None` is the executable's name.
    pub app: Option<String>,
    /// Show a window only after its first frame is presented. Default: on
    /// Windows only (see the module doc for the other platforms).
    pub hold_until_first_frame: bool,
    /// Draw every pipeline once offscreen during the prewarm (drivers that
    /// finish compiling at the first draw; skipped on software adapters).
    pub warm_up_draw: bool,
    /// Pipeline cache base directory instead of the platform cache directory
    /// (Android: the activity's internal data path, set by the host).
    pub cache_base: Option<PathBuf>,
}

impl Default for PrewarmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pipeline_cache: true,
            app: None,
            hold_until_first_frame: cfg!(target_os = "windows"),
            warm_up_draw: true,
            cache_base: None,
        }
    }
}

impl PrewarmConfig {
    /// No prewarm, no pipeline cache, windows shown as before.
    pub fn off() -> Self {
        Self { enabled: false, pipeline_cache: false, hold_until_first_frame: false, warm_up_draw: false, ..Self::default() }
    }

    /// `enabled` after the `UZOR_PREWARM` override (`0` / `off` / `false`).
    pub(crate) fn effective(mut self) -> Self {
        if let Ok(v) = std::env::var("UZOR_PREWARM") {
            if matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false" | "no") {
                self.enabled = false;
                self.hold_until_first_frame = false;
            }
        }
        if !self.enabled {
            self.pipeline_cache = false;
        }
        self
    }

    /// The directory the hub keeps this app's pipeline cache in.
    pub(crate) fn cache_dir(&self) -> Option<PathBuf> {
        if !self.pipeline_cache {
            return None;
        }
        let base = self.cache_base.clone().or_else(super::window_create::platform_pipeline_cache_dir)?;
        let app = self.app.clone().unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "app".into())
        });
        Some(uzor_render_hub::app_pipeline_cache_dir(&base, &app))
    }

    pub(crate) fn options(&self, hub: &RenderHub) -> PrewarmOptions {
        PrewarmOptions {
            backend: hub.active(),
            msaa_samples: hub.settings().msaa_samples,
            pipeline_cache_dir: self.cache_dir(),
            warm_up_draw: self.warm_up_draw,
        }
    }
}

/// The startup thread: the hub, then the prewarm for its active backend.
pub(crate) struct Startup(JoinHandle<(RenderHub, Option<Prewarmed>)>);

impl Startup {
    pub(crate) fn spawn(preferred: Option<RenderBackend>, cfg: PrewarmConfig) -> Self {
        Self(std::thread::spawn(move || {
            let hub = super::window_create::build_hub(preferred);
            trace("hub", &format!("{:?}", hub.active()));
            let warm = warm(&hub, &cfg, "startup");
            (hub, warm)
        }))
    }

    /// Wait for the thread. `None` when it panicked (the caller builds the hub itself).
    pub(crate) fn join(self) -> Option<(RenderHub, Option<Prewarmed>)> {
        let t = Instant::now();
        let r = self.0.join().ok();
        trace("joined", &format!("waited {:.1} ms", t.elapsed().as_secs_f64() * 1e3));
        r
    }
}

/// Prewarm `hub`'s active backend now, if it has anything to warm.
pub(crate) fn warm(hub: &RenderHub, cfg: &PrewarmConfig, why: &str) -> Option<Prewarmed> {
    if !cfg.enabled || !uzor_render_hub::prewarm_supported(hub.active()) {
        return None;
    }
    match uzor_render_hub::prewarm(&cfg.options(hub)) {
        Ok(p) => {
            let t = p.timings();
            trace(
                "prewarmed",
                &format!(
                    "{why} {:?}: device {:.1} ms, pipelines {:.1} ms, warm-up draw {:.1} ms, cache {} B loaded",
                    p.backend(),
                    t.device.as_secs_f64() * 1e3,
                    t.pipelines.as_secs_f64() * 1e3,
                    t.draw.as_secs_f64() * 1e3,
                    t.cache_bytes_loaded
                ),
            );
            Some(p)
        }
        Err(e) => {
            log::warn!("prewarm ({why}) failed, the window takes the factory path: {e}");
            trace("prewarm-failed", &e.to_string());
            None
        }
    }
}

/// Write the pipeline cache off the UI thread (once, after a first frame).
pub(crate) fn save_cache(store: PipelineCacheStore) {
    std::thread::spawn(move || match store.save_if_changed() {
        Ok(0) => {}
        Ok(n) => trace("pipeline-cache", &format!("saved {n} B to {:?}", store.path())),
        Err(e) => log::warn!("pipeline cache not saved: {e}"),
    });
}

fn origin() -> Instant {
    static T0: OnceLock<Instant> = OnceLock::new();
    *T0.get_or_init(Instant::now)
}

/// Start the `UZOR_TRACE_STARTUP` clock (called first thing in `run_native`).
pub(crate) fn mark_origin() {
    let _ = origin();
}

/// `UZOR_TRACE_STARTUP=1`: one stderr line per startup stage, ms since `run_native`.
pub(crate) fn trace(stage: &str, detail: &str) {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("UZOR_TRACE_STARTUP").is_some_and(|v| !v.is_empty() && v != "0")) {
        eprintln!("[startup] {:9.1} ms {stage} {detail}", origin().elapsed().as_secs_f64() * 1e3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_on_and_off_is_off() {
        let d = PrewarmConfig::default();
        assert!(d.enabled && d.pipeline_cache);
        assert_eq!(d.hold_until_first_frame, cfg!(target_os = "windows"));
        let o = PrewarmConfig::off();
        assert!(!o.enabled && !o.pipeline_cache && !o.hold_until_first_frame);
        assert!(o.effective().cache_dir().is_none());
    }

    #[test]
    fn cache_dir_is_under_uzor_and_the_app() {
        let c = PrewarmConfig { app: Some("hq".into()), cache_base: Some(PathBuf::from("/tmp/x")), ..Default::default() };
        assert_eq!(c.cache_dir(), Some(PathBuf::from("/tmp/x/uzor/hq")));
        let off = PrewarmConfig { pipeline_cache: false, ..c };
        assert_eq!(off.cache_dir(), None);
    }
}
