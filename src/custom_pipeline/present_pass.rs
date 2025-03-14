use wgpu::PipelineCompilationOptions;

use super::gpu_buffers::GpuBuffers;

pub struct PresentPass {
    pipeline: wgpu::RenderPipeline,
    bind_group_0: wgpu::BindGroup,
    bind_group_1: wgpu::BindGroup,
}

impl PresentPass {
    pub fn new(device: &wgpu::Device, buffers: &GpuBuffers) -> Self {
        // A simple sampler for reading the output texture
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        // We create a bind group layout that has:
        //   binding(0) -> our output texture (sampled)
        //   binding(1) -> a sampler
        let bind_group_layout_0 =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PresentPass BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let bind_group_layout_1 =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Screen Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PresentPass bind group"),
            layout: &bind_group_layout_0,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&buffers.output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Present Pass: Group1"),
            layout: &bind_group_layout_1,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.screen_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PresentPass pipeline layout"),
            bind_group_layouts: &[&bind_group_layout_0, &bind_group_layout_1],
            push_constant_ranges: &[],
        });

        let shader_source = include_str!("shaders/present.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raster Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PresentPass pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_0,
            bind_group_1,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_view: &wgpu::TextureView) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        self.bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PresentPass bind group"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(new_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
    }

    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("PresentPass RenderPass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group_0, &[]);
        rpass.set_bind_group(1, &self.bind_group_1, &[]);
        // Draw a single full screen triangle
        rpass.draw(0..3, 0..1);
    }
}
