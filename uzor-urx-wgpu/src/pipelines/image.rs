//! Image pipeline — textured quad whose four corners are the full affine
//! of the dest rect (shear and reflection included). Samples a
//! per-`ImageId` texture from `crate::image_cache::NativeImageCache`.
//! `DrawCommand::Image` only — NOT `peniko::Brush::Image`.
//!
//! `ImageInstance` is 72 bytes. The WGSL `IMAGE_SHADER_NATIVE` mirrors it.
//! Corner order is p00, p10, p01, p11 (local (x0,y0), (x1,y0), (x0,y1), (x1,y1)).

use bytemuck::{Pod, Zeroable};

/// Straight white at alpha `A`. `unpack4x8unorm` puts R in the low byte,
/// so the packed word is `(A << 24) | 0x00FF_FFFF`. Alpha 1 => `0xFFFFFFFF`.
pub(crate) fn pack_white_alpha(alpha: f32) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    (a.min(255) << 24) | 0x00FF_FFFF
}

/// A textured-quad image instance — 72 bytes, tightly packed.
///
/// Memory layout:
/// - c0, c1, c2, c3: 8 bytes each (offsets 0, 8, 16, 24) — screen-space corners p00, p10, p01, p11
/// - uv_pos, uv_size: 8 bytes each (offsets 32, 40)
/// - tint: 4 bytes (offset 48, u32) + pad: 4 bytes (offset 52)
/// - clip_rect: 16 bytes (offset 56)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct ImageInstance {
    pub c0: [f32; 2],
    pub c1: [f32; 2],
    pub c2: [f32; 2],
    pub c3: [f32; 2],
    pub uv_pos: [f32; 2],
    pub uv_size: [f32; 2],
    pub tint: u32,
    pub _pad: u32,
    pub clip_rect: [f32; 4],
}

const _: () = assert!(
    std::mem::size_of::<ImageInstance>() == 72,
    "ImageInstance must stay 72 bytes — WGSL struct in shaders.rs mirrors this layout"
);

/// Vertex buffer layout for `ImageInstance`. The pad at offset 52 is not
/// an attribute; `clip_rect` starts at 56.
pub(crate) fn image_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    use wgpu::VertexFormat::*;

    static ATTRS: &[wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute { shader_location: 0, format: Float32x2, offset: 0 },
        wgpu::VertexAttribute { shader_location: 1, format: Float32x2, offset: 8 },
        wgpu::VertexAttribute { shader_location: 2, format: Float32x2, offset: 16 },
        wgpu::VertexAttribute { shader_location: 3, format: Float32x2, offset: 24 },
        wgpu::VertexAttribute { shader_location: 4, format: Float32x2, offset: 32 },
        wgpu::VertexAttribute { shader_location: 5, format: Float32x2, offset: 40 },
        wgpu::VertexAttribute { shader_location: 6, format: Uint32, offset: 48 },
        wgpu::VertexAttribute { shader_location: 7, format: Float32x4, offset: 56 },
    ];

    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ImageInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: ATTRS,
    }
}

/// Same initial capacity constant as every sibling pipeline.
const INITIAL_CAPACITY: usize = 1024;

fn make_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uzor_urx_wgpu.native_image_instance_buffer"),
        size: (capacity * std::mem::size_of::<ImageInstance>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Owns the image `wgpu::RenderPipeline` pair + the shared grow-only
/// instance buffer. Per-image textures live in
/// `crate::image_cache::NativeImageCache` and are bound per batch.
pub(crate) struct ImagePipeline {
    pipeline_off: wgpu::RenderPipeline,
    pipeline_test: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    capacity: usize,
}

impl ImagePipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        uniform_bgl: &wgpu::BindGroupLayout,
        image_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("uzor_urx_wgpu.image_shader_native"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::IMAGE_SHADER_NATIVE.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("uzor_urx_wgpu.image_pipeline_layout_native"),
            bind_group_layouts: &[Some(uniform_bgl), Some(image_bgl)],
            immediate_size: 0,
        });
        let vertex_buffers = [image_instance_layout()];
        let color_targets = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(crate::pipelines::quad::premultiplied_blend_state()),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let (pipeline_off, pipeline_test) =
            crate::pipelines::build_off_test_pair(device, |depth_stencil| wgpu::RenderPipelineDescriptor {
                label: Some("uzor_urx_wgpu.image_pipeline_native"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &color_targets,
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil,
                multisample: wgpu::MultisampleState { count: sample_count, ..Default::default() },
                multiview_mask: None,
                cache: None,
            });
        let buffer = make_instance_buffer(device, INITIAL_CAPACITY);

        Self { pipeline_off, pipeline_test, buffer, capacity: INITIAL_CAPACITY }
    }

    /// Upload `data`, growing the buffer (doubling capacity) if it no
    /// longer fits. No-op on an empty slice.
    pub(crate) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[ImageInstance]) {
        if data.is_empty() {
            return;
        }
        let needed = data.len();
        if needed > self.capacity {
            while self.capacity < needed {
                self.capacity *= 2;
            }
            self.buffer = make_instance_buffer(device, self.capacity);
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    /// Bind this pipeline + its vertex buffer + `image_bind_group` at
    /// group 1 onto `pass`. Called once per `BatchKind::Image(id)` batch.
    pub(crate) fn bind(&self, pass: &mut wgpu::RenderPass<'_>, stencil_ref: Option<u32>, image_bind_group: &wgpu::BindGroup) {
        match stencil_ref {
            None => pass.set_pipeline(&self.pipeline_off),
            Some(r) => {
                pass.set_pipeline(&self.pipeline_test);
                pass.set_stencil_reference(r);
            }
        }
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        pass.set_bind_group(1, image_bind_group, &[]);
    }

    /// Draw instances `[start, start + count)` — 6 procedurally
    /// generated vertices per instance (2 triangles).
    pub(crate) fn draw_range(&self, pass: &mut wgpu::RenderPass<'_>, start: u32, count: u32) {
        pass.draw(0..6, start..(start + count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_instance_is_72_bytes() {
        assert_eq!(std::mem::size_of::<ImageInstance>(), 72);
    }

    #[test]
    fn pack_white_alpha_opaque_is_all_ones() {
        assert_eq!(pack_white_alpha(1.0), 0xFFFF_FFFF);
        assert_eq!(pack_white_alpha(0.0), 0x00FF_FFFF);
    }
}
