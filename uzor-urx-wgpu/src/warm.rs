//! Warm-up draw for [`NativeUrxRenderer::warm_up`].
//!
//! Some drivers compile a pipeline only when it first draws (llvmpipe JITs
//! its fragment variants per draw state; others defer part of the backend
//! compile). Drawing one small scene that touches every pipeline the 2D UI
//! uses, offscreen, moves that cost off the first visible frame.

use uzor_urx_core::math::{Affine, BezPath, BlendMode, Brush, Color, Gradient, Rect, RoundedRect, Vec2};
use uzor_urx_core::scene::{DrawCommand, FillRule, Scene, Stroke};

use crate::renderer::{NativeUrxRenderer, Viewport};

/// Side of the offscreen warm-up target, px.
pub(crate) const SIDE: u32 = 64;

/// Fills (sharp, rounded, linear and radial gradients), a stroke, a line,
/// a tessellated path fill and stroke, a rounded clip (stencil) and a blend
/// layer (composite pass).
pub(crate) fn scene() -> Scene {
    let solid = |r, g, b| Brush::Solid(Color::from_rgba8(r, g, b, 255));
    let id = Affine::IDENTITY;
    let mut s = Scene::new();
    s.push(DrawCommand::FillRect { rect: Rect::new(0.0, 0.0, 64.0, 64.0), radii: None, brush: solid(20, 20, 20), transform: id });
    s.push(DrawCommand::FillRect { rect: Rect::new(2.5, 2.5, 30.5, 30.5), radii: Some([6.0; 4]), brush: solid(200, 60, 60), transform: id });
    s.push(DrawCommand::StrokeRect { rect: Rect::new(4.5, 34.5, 28.5, 60.5), radii: Some([3.0; 4]), stroke: Stroke::default(), brush: solid(60, 200, 60), transform: id });
    s.push(DrawCommand::Line { from: Vec2::new(32.0, 2.0), to: Vec2::new(62.0, 30.0), stroke: Stroke::default(), brush: solid(60, 60, 200), transform: id });
    let mut tri = BezPath::new();
    tri.move_to((34.5, 34.5));
    tri.quad_to((62.5, 36.5), (48.5, 62.5));
    tri.close_path();
    s.push(DrawCommand::FillPath { path: tri.clone(), rule: FillRule::NonZero, brush: solid(200, 200, 60), transform: id });
    s.push(DrawCommand::StrokePath { path: tri, stroke: Stroke::default(), brush: solid(60, 200, 200), transform: id });
    let lin = Gradient::new_linear((32.0, 0.0), (64.0, 0.0))
        .with_stops([(0.0f32, Color::from_rgba8(255, 0, 0, 255)), (1.0f32, Color::from_rgba8(0, 0, 255, 255))]);
    s.push(DrawCommand::FillRect { rect: Rect::new(32.5, 2.5, 62.5, 14.5), radii: Some([2.0; 4]), brush: Brush::Gradient(lin), transform: id });
    let rad = Gradient::new_radial((47.0, 22.0), 8.0)
        .with_stops([(0.0f32, Color::from_rgba8(255, 255, 255, 255)), (1.0f32, Color::from_rgba8(0, 0, 0, 255))]);
    s.push(DrawCommand::FillRect { rect: Rect::new(38.5, 15.5, 56.5, 30.5), radii: None, brush: Brush::Gradient(rad), transform: id });
    s.push(DrawCommand::PushClipRoundedRect { rect: RoundedRect::new(8.5, 8.5, 56.5, 56.5, 10.0), transform: id });
    s.push(DrawCommand::PushBlendLayer { mode: BlendMode::default(), alpha: 0.5, transform: id });
    s.push(DrawCommand::FillRect { rect: Rect::new(0.0, 0.0, 64.0, 64.0), radii: None, brush: solid(90, 90, 160), transform: id });
    s.push(DrawCommand::PopBlendLayer);
    s.push(DrawCommand::PopClip);
    s
}

impl NativeUrxRenderer {
    /// Draw a small scene that uses every 2D pipeline into an offscreen
    /// target and wait for the GPU, so drivers that finish compiling a
    /// pipeline at its first draw do it here, not in the first visible
    /// frame. Leaves no visible state behind (the retained frame is
    /// dropped; caches keep a few small entries). Returns the target
    /// texture (`format()`, 64×64) for callers that want to warm a blit
    /// from it as well.
    pub fn warm_up(&mut self) -> wgpu::Texture {
        let device = self.device.clone();
        let queue = self.queue.clone();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("uzor_urx_wgpu.warm_up_target"),
            size: wgpu::Extent3d { width: SIDE, height: SIDE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("uzor_urx_wgpu.warm_up") });
        if let Err(e) = self.render_into_encoder(&scene(), &mut encoder, &view, Viewport { width: SIDE, height: SIDE }) {
            eprintln!("[uzor-urx-wgpu] warm-up draw failed: {e:?}");
        }
        queue.submit([encoder.finish()]);
        self.warm_up_textured(&view, SIDE, SIDE);
        let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        target
    }
}
