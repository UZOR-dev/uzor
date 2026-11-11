//! Renderer prewarm: the GPU work a window's first frame waits on, done
//! before the window exists.
//!
//! A cold first frame pays for the wgpu instance, the adapter and the device,
//! and then for every render pipeline the backend builds (the URX native
//! pipelines, or vello's compute pipelines). None of that needs a surface, so
//! [`prewarm`] runs it on any thread (the native host starts it on a
//! background thread before its event loop) and [`Prewarmed::into_render_state`]
//! later binds the result to the window: surface, configure, target texture,
//! blitter, a few milliseconds.
//!
//! The pipelines go through a `wgpu::PipelineCache` (Vulkan; other backends
//! have none and skip it) loaded from `<cache_dir>/<backend>-<version>/pipeline-cache/`,
//! so a later cold start does not compile shaders again. The host saves it
//! after the first frame ([`PipelineCacheStore::save_if_changed`]).
//!
//! Covered: [`RenderBackend::VelloGpu`] (device + `vello::Renderer`), the URX
//! family (device; plus the native renderer for [`RenderBackend::UrxWgpu`]).
//! Any other backend answers [`SurfaceError::UnsupportedBackend`] and the
//! caller takes its factory path, as do surfaces the prewarmed adapter cannot
//! present to.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use uzor::layout::window::RawHandle;

use crate::factory::GpuDevicePool;
use crate::{RenderBackend, SurfaceError, SurfaceSize, WindowRenderState};

/// What [`prewarm`] warms and where it keeps the pipeline cache.
#[derive(Clone, Debug)]
pub struct PrewarmOptions {
    /// The backend the window will render with (`RenderHub::active`).
    pub backend: RenderBackend,
    /// `PerfSettings::msaa_samples`: the URX pipelines bake the sample count in.
    pub msaa_samples: u8,
    /// Base directory of the pipeline cache, e.g. `<platform cache>/uzor/<app>`.
    /// `None` keeps the cache in memory for this run only.
    pub pipeline_cache_dir: Option<PathBuf>,
    /// Draw every pipeline once offscreen and blit onto the likely
    /// swapchain formats, for drivers that finish compiling a pipeline at
    /// its first draw. Skipped on software adapters.
    pub warm_up_draw: bool,
}

/// Wall-clock cost of each [`prewarm`] stage.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrewarmTimings {
    /// Instance + adapter + device.
    pub device: Duration,
    /// Pipeline cache load and the backend's renderer (its pipelines).
    pub pipelines: Duration,
    /// The warm-up draw: every pipeline drawn once offscreen, then the
    /// swapchain blit (drivers that finish compiling at the first draw).
    /// Zero when skipped.
    pub draw: Duration,
    /// Pipeline-cache bytes read from disk (0: cold, or no cache).
    pub cache_bytes_loaded: usize,
}

/// A loaded `wgpu::PipelineCache` and where it is saved.
pub struct PipelineCacheStore {
    cache: wgpu::PipelineCache,
    info: wgpu::AdapterInfo,
    dir: Option<PathBuf>,
    key: String,
    loaded: usize,
}

impl PipelineCacheStore {
    /// The cache to build pipelines through.
    pub fn cache(&self) -> &wgpu::PipelineCache {
        &self.cache
    }

    /// The blob's path, when the cache is persisted.
    pub fn path(&self) -> Option<PathBuf> {
        self.dir.as_deref().map(|d| uzor_urx_core::pipeline_cache::cache_path_for_adapter(d, &self.key, &self.info))
    }

    /// Write the cache when it holds something the file on disk does not
    /// (a cold run, a driver that added pipelines). Bytes written, `0` when
    /// nothing changed or nothing is persisted.
    pub fn save_if_changed(&self) -> std::io::Result<usize> {
        let Some(dir) = self.dir.as_deref() else { return Ok(0) };
        match self.cache.get_data() {
            Some(d) if d.len() != self.loaded => {
                uzor_urx_core::pipeline_cache::save_to_disk(&self.cache, Some(dir), &self.info, &self.key)
            }
            _ => Ok(0),
        }
    }
}

// Built once and moved into the window state; boxing buys nothing.
#[allow(clippy::large_enum_variant)]
enum Warm {
    /// Device only: the backend builds its renderer lazily (URX CPU / hybrid / full).
    Device,
    #[cfg(feature = "vello-gpu")]
    Vello(vello::Renderer),
    #[cfg(feature = "urx")]
    Urx(uzor_urx_wgpu::NativeUrxRenderer),
}

/// The device and renderer of one window, built before the window.
pub struct Prewarmed {
    backend: RenderBackend,
    gpu_pool: GpuDevicePool,
    dev_id: usize,
    warm: Warm,
    /// Blitters onto the likely swapchain formats, each already drawn once.
    blitters: Vec<(wgpu::TextureFormat, wgpu::util::TextureBlitter)>,
    cache: Option<PipelineCacheStore>,
    timings: PrewarmTimings,
}

impl std::fmt::Debug for Prewarmed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prewarmed").field("backend", &self.backend).field("timings", &self.timings).finish_non_exhaustive()
    }
}

/// Whether [`prewarm`] has anything to do for `backend` in this build.
pub fn prewarm_supported(backend: RenderBackend) -> bool {
    match backend {
        #[cfg(feature = "vello-gpu")]
        RenderBackend::VelloGpu => true,
        #[cfg(feature = "urx")]
        RenderBackend::UrxCpu | RenderBackend::UrxWgpu | RenderBackend::UrxHybrid | RenderBackend::UrxWgpuFull => true,
        _ => false,
    }
}

/// Build the device (and, where the backend has one up front, the renderer
/// with all its pipelines) for `opts.backend`. Blocking; any thread.
pub fn prewarm(opts: &PrewarmOptions) -> Result<Prewarmed, SurfaceError> {
    if !prewarm_supported(opts.backend) {
        return Err(SurfaceError::UnsupportedBackend(opts.backend));
    }
    let t0 = Instant::now();
    let ready = crate::factories::prewarm_device()?;
    let device_t = t0.elapsed();
    let (gpu_pool, dev_id) = (ready.gpu_pool, ready.dev_id);

    let t1 = Instant::now();
    let cache = load_cache(&gpu_pool, dev_id, opts);
    let pc = cache.as_ref().map(|c| c.cache.clone());
    let warm = {
        let dh = &gpu_pool.devices[dev_id];
        match opts.backend {
            #[cfg(feature = "vello-gpu")]
            RenderBackend::VelloGpu => Warm::Vello(crate::factories::build_renderer_with_cache(&dh.device, pc)?),
            #[cfg(feature = "urx")]
            RenderBackend::UrxWgpu => Warm::Urx(uzor_urx_wgpu::NativeUrxRenderer::with_pipeline_cache(
                dh.device.clone(),
                dh.queue.clone(),
                wgpu::TextureFormat::Rgba8Unorm,
                crate::submit_urx::native_urx_sample_count(opts.msaa_samples),
                pc.as_ref(),
            )),
            _ => Warm::Device,
        }
    };
    let pipelines_t = t1.elapsed();
    let t2 = Instant::now();
    let mut warm = warm;
    // On a software adapter (llvmpipe, WARP) the URX warm-up scene JITs
    // every variant it touches, most of which a first frame never uses:
    // measured on llvmpipe it cost more than it saved, so there only the
    // blit is warmed. vello's warm-up stays (its first dispatch compiles
    // the whole compute chain, seconds on llvmpipe).
    let software = gpu_pool.devices[dev_id].adapter().get_info().device_type == wgpu::DeviceType::Cpu;
    let blitters = if opts.warm_up_draw {
        #[cfg(feature = "urx")]
        let draw_renderer = !(software && matches!(warm, Warm::Urx(_)));
        #[cfg(not(feature = "urx"))]
        let draw_renderer = { let _ = software; true };
        warm_up_draw(&gpu_pool, dev_id, &mut warm, draw_renderer)
    } else {
        Vec::new()
    };
    let timings = PrewarmTimings {
        device: device_t,
        pipelines: pipelines_t,
        draw: t2.elapsed(),
        cache_bytes_loaded: cache.as_ref().map_or(0, |c| c.loaded),
    };
    Ok(Prewarmed { backend: opts.backend, gpu_pool, dev_id, warm, blitters, cache, timings })
}

/// The swapchain formats the surface code picks (first non-sRGB 8-bit RGBA/BGRA).
const BLIT_FORMATS: [wgpu::TextureFormat; 2] = [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba8Unorm];

/// Draw once with the renderer (when it has one) and blit the result onto
/// each likely swapchain format, offscreen, and wait for the GPU.
fn warm_up_draw(
    gpu_pool: &GpuDevicePool,
    dev_id: usize,
    warm: &mut Warm,
    draw_renderer: bool,
) -> Vec<(wgpu::TextureFormat, wgpu::util::TextureBlitter)> {
    const SIDE: u32 = 64;
    let dh = &gpu_pool.devices[dev_id];
    let (device, queue) = (&dh.device, &dh.queue);
    let tex = |format, usage, label| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: SIDE, height: SIDE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    // The surface's target texture: Rgba8Unorm, the renderer draws into it.
    let plain = || tex(wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::TEXTURE_BINDING, "uzor-render-hub.warm-up-target");
    let src = match warm {
        _ if !draw_renderer => plain(),
        #[cfg(feature = "urx")]
        Warm::Urx(r) => r.warm_up(),
        #[cfg(feature = "vello-gpu")]
        Warm::Vello(r) => {
            let t = tex(
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                "uzor-render-hub.warm-up-target",
            );
            let mut scene = vello::Scene::new();
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::from_rgba8(200, 60, 60, 255),
                None,
                &vello::kurbo::RoundedRect::new(4.0, 4.0, 60.0, 60.0, 8.0),
            );
            let params = vello::RenderParams {
                base_color: vello::peniko::Color::from_rgba8(20, 20, 20, 255),
                width: SIDE,
                height: SIDE,
                antialiasing_method: vello::AaConfig::Area,
            };
            if let Err(e) = r.render_to_texture(device, queue, &scene, &t.create_view(&Default::default()), &params) {
                eprintln!("[render-hub] prewarm: vello warm-up draw failed: {e}");
            }
            t
        }
        Warm::Device => plain(),
    };
    let src_view = src.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("uzor-render-hub.warm-up-blit") });
    let mut blitters = Vec::new();
    let mut dsts = Vec::new();
    for format in BLIT_FORMATS {
        let b = wgpu::util::TextureBlitter::new(device, format);
        let dst = tex(format, wgpu::TextureUsages::RENDER_ATTACHMENT, "uzor-render-hub.warm-up-blit");
        b.copy(device, &mut encoder, &src_view, &dst.create_view(&wgpu::TextureViewDescriptor::default()));
        dsts.push(dst);
        blitters.push((format, b));
    }
    queue.submit([encoder.finish()]);
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
    blitters
}

fn load_cache(gpu_pool: &GpuDevicePool, dev_id: usize, opts: &PrewarmOptions) -> Option<PipelineCacheStore> {
    let dh = &gpu_pool.devices[dev_id];
    // The device asks for PIPELINE_CACHE only where the adapter has it
    // (Vulkan); creating a cache without it panics.
    if !dh.device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        return None;
    }
    let info = dh.adapter().get_info();
    let key = format!("{}-{}", opts.backend.as_str(), env!("CARGO_PKG_VERSION"));
    let data = opts
        .pipeline_cache_dir
        .as_deref()
        .map(|d| uzor_urx_core::pipeline_cache::cache_path_for_adapter(d, &key, &info))
        .and_then(|p| std::fs::read(p).ok());
    let loaded = data.as_ref().map_or(0, Vec::len);
    // SAFETY: the blob is opaque driver data; `fallback: true` makes wgpu
    // discard a corrupt or foreign one and start empty instead of failing.
    let cache = unsafe {
        dh.device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("uzor-render-hub.pipeline-cache"),
            data: data.as_deref(),
            fallback: true,
        })
    };
    Some(PipelineCacheStore { cache, info, dir: opts.pipeline_cache_dir.clone(), key, loaded })
}

impl Prewarmed {
    /// The backend this was warmed for.
    pub fn backend(&self) -> RenderBackend {
        self.backend
    }

    /// Stage timings.
    pub fn timings(&self) -> PrewarmTimings {
        self.timings
    }

    /// The pipeline cache, when the device has one.
    pub fn pipeline_cache(&self) -> Option<&PipelineCacheStore> {
        self.cache.as_ref()
    }

    /// Bind to the window behind `handle`: the surface on the prewarmed
    /// device, configured once at `size`, with the renderer in place. Also
    /// hands back the pipeline cache for the host to save after the first
    /// frame. Fails (the caller takes the factory path) when the prewarmed
    /// adapter cannot present to this surface.
    pub fn into_render_state(
        self,
        handle: &RawHandle,
        size: SurfaceSize,
    ) -> Result<(WindowRenderState, Option<PipelineCacheStore>), SurfaceError> {
        let Prewarmed { backend, gpu_pool, dev_id, warm, blitters, cache, .. } = self;
        let ready = crate::factories::GpuDeviceReady { gpu_pool, dev_id };
        let (gpu_pool, mut surface, dev_id) = crate::factories::build_surface_from_device(ready, handle, size)?;
        // The blitter that already drew once onto this format.
        if let Some((_, b)) = blitters.into_iter().find(|(f, _)| *f == surface.format) {
            surface.blitter = b;
        }
        let mut state = match warm {
            #[cfg(feature = "vello-gpu")]
            Warm::Vello(renderer) => WindowRenderState::new_gpu(gpu_pool, surface, renderer, dev_id),
            #[cfg(feature = "urx")]
            Warm::Urx(renderer) => {
                let mut s = WindowRenderState::new_gpu_no_vello(gpu_pool, surface, dev_id, backend, 1.0);
                s.urx_native_renderer = Some(renderer);
                s
            }
            Warm::Device => WindowRenderState::new_gpu_no_vello(gpu_pool, surface, dev_id, backend, 1.0),
        };
        state.resize_surface(size.width, size.height);
        Ok((state, cache))
    }
}

/// `<base>/uzor/<app>`: where a host keeps `app`'s pipeline cache under the
/// platform cache directory `base`.
pub fn app_pipeline_cache_dir(base: &Path, app: &str) -> PathBuf {
    let app: String = app.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    base.join("uzor").join(if app.is_empty() { "app".into() } else { app })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_dir_is_namespaced_and_flattened() {
        let d = app_pipeline_cache_dir(Path::new("/c"), "hq uzor/x");
        assert_eq!(d, PathBuf::from("/c/uzor/hq_uzor_x"));
        assert_eq!(app_pipeline_cache_dir(Path::new("/c"), ""), PathBuf::from("/c/uzor/app"));
    }

    #[test]
    fn unsupported_backends_are_refused_without_touching_the_gpu() {
        for b in [RenderBackend::TinySkia, RenderBackend::VelloCpu, RenderBackend::Canvas2d] {
            assert!(!prewarm_supported(b));
            let e = prewarm(&PrewarmOptions { backend: b, msaa_samples: 0, pipeline_cache_dir: None, warm_up_draw: true }).unwrap_err();
            assert!(matches!(e, SurfaceError::UnsupportedBackend(x) if x == b));
        }
    }
}
