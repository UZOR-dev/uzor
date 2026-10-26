//! Textured-triangle pipeline for `DrawCommand::ImageTri`.
//!
//! Same bind group layout as [`super::image`] (uniform group 0,
//! texture+sampler group 1), same premul blend, same `_off`/`_test`
//! stencil pair. One instance is one triangle (3 vertices).

use bytemuck::{Pod, Zeroable};

/// One textured triangle. 72 bytes, tightly packed.
///
/// - p0, p1, p2: 8 bytes each (offsets 0, 8, 16) — screen positions
/// - uv0, uv1, uv2: 8 bytes each (offsets 24, 32, 40)
/// - tint: 4 bytes (offset 48) + pad: 4 bytes (offset 52)
/// - clip_rect: 16 bytes (offset 56)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct ImageTriInstance {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub p2: [f32; 2],
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub tint: u32,
    pub _pad: u32,
    pub clip_rect: [f32; 4],
}

const _: () = assert!(
    std::mem::size_of::<ImageTriInstance>() == 72,
    "ImageTriInstance must stay 72 bytes — WGSL struct in shaders.rs mirrors this layout"
);

pub(crate) fn image_tri_instance_layout() -> wgpu::VertexBufferLayout<'static> {
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
        array_stride: std::mem::size_of::<ImageTriInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: ATTRS,
    }
}

const INITIAL_CAPACITY: usize = 1024;

fn make_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uzor_urx_wgpu.native_image_tri_instance_buffer"),
        size: (capacity * std::mem::size_of::<ImageTriInstance>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub(crate) struct ImageTriPipeline {
    pipeline_off: wgpu::RenderPipeline,
    pipeline_test: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    capacity: usize,
}

impl ImageTriPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        uniform_bgl: &wgpu::BindGroupLayout,
        image_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("uzor_urx_wgpu.image_tri_shader_native"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::IMAGE_TRI_SHADER_NATIVE.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("uzor_urx_wgpu.image_tri_pipeline_layout_native"),
            bind_group_layouts: &[Some(uniform_bgl), Some(image_bgl)],
            immediate_size: 0,
        });
        let vertex_buffers = [image_tri_instance_layout()];
        let color_targets = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(crate::pipelines::quad::premultiplied_blend_state()),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let (pipeline_off, pipeline_test) =
            crate::pipelines::build_off_test_pair(device, |depth_stencil| wgpu::RenderPipelineDescriptor {
                label: Some("uzor_urx_wgpu.image_tri_pipeline_native"),
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

    pub(crate) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[ImageTriInstance]) {
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

    /// 3 vertices per instance.
    pub(crate) fn draw_range(&self, pass: &mut wgpu::RenderPass<'_>, start: u32, count: u32) {
        pass.draw(0..3, start..(start + count));
    }
}
