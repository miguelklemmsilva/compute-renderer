use super::{util::create_buffer_bind_group_layout_entry, GpuBuffers};

pub struct FragmentPass {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
    pub bind_group_2: wgpu::BindGroup,
    pub bind_group_3: wgpu::BindGroup,
    pub bind_group_4: wgpu::BindGroup,
    pub bind_group_5: wgpu::BindGroup,
}

impl FragmentPass {
    pub fn new(device: &wgpu::Device, buffers: &GpuBuffers) -> Self {
        let group0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Pass: Group0 Layout (Output)"),
            entries: &[
                create_buffer_bind_group_layout_entry(0, false),
            ],
        });

        let group1_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Pass: Group1 Layout (Screen)"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let group2_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Pass: Group2 Layout (Camera)"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let group3_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Pass: Group3 Layout (Lights)"),
            entries: &[create_buffer_bind_group_layout_entry(0, true)],
        });

        let group4_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Pass: Group4 Layout (Effect)"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let group5_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Pass: Group5 Layout (Fragments)"),
            entries: &[create_buffer_bind_group_layout_entry(0, false)],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fragment Pipeline Layout"),
            bind_group_layouts: &[
                &group0_layout,
                &group1_layout,
                &group2_layout,
                &group3_layout,
                &group4_layout,
                &group5_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/fragment.wgsl"));

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fragment Pass Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("fragment_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group0"),
            layout: &group0_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.output_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group1"),
            layout: &group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.screen_buffer.as_entire_binding(),
            }],
        });

        let bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group2"),
            layout: &group2_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.camera_buffer.as_entire_binding(),
            }],
        });

        let bind_group_3 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group3"),
            layout: &group3_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.light_buffer.as_entire_binding(),
            }],
        });

        let bind_group_4 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group4"),
            layout: &group4_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.effect_buffer.as_entire_binding(),
            }],
        });

        let bind_group_5 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group5"),
            layout: &group5_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.fragment_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            bind_group_0,
            bind_group_1,
            bind_group_2,
            bind_group_3,
            bind_group_4,
            bind_group_5,
        }
    }

    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder, total_pixel_dispatch: u32) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Fragment Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &self.bind_group_0, &[]);
        cpass.set_bind_group(1, &self.bind_group_1, &[]);
        cpass.set_bind_group(2, &self.bind_group_2, &[]);
        cpass.set_bind_group(3, &self.bind_group_3, &[]);
        cpass.set_bind_group(4, &self.bind_group_4, &[]);
        cpass.set_bind_group(5, &self.bind_group_5, &[]);

        cpass.dispatch_workgroups(total_pixel_dispatch, 1, 1);
    }
}
