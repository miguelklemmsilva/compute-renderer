pub struct RasterPass {
    pub pipeline: wgpu::ComputePipeline,
}

impl RasterPass {
    pub fn new(device: &wgpu::Device) -> Self {
        // Bind Group Layout for Output Buffer
        let color_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Output Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Bind Group Layout for Depth Buffer
        let depth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Depth Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Bind Group Layout for Uniforms
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Uniform Bind Group Layout"),
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

        // Bind Group Layout for Vertex Buffer
        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Raster: Vertex Buffer Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create Pipeline Layout with all Bind Group Layouts
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raster Pipeline Layout"),
            bind_group_layouts: &[
                &color_bind_group_layout,
                &depth_bind_group_layout,
                &uniform_bind_group_layout,
                &vertex_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        // Create Shader Module
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/raster.wgsl"));

        // Create Compute Pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Raster Pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Option::Some("raster"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}

pub struct RasterBindings {
    pub output_buffer: wgpu::BindGroup,
    pub depth_buffer: wgpu::BindGroup,
    pub uniform: wgpu::BindGroup,
    pub vertex_buffer: wgpu::BindGroup,
}

impl RasterBindings {
    pub fn new(
        device: &wgpu::Device,
        RasterPass { pipeline }: &RasterPass,
        output_buffer: &wgpu::Buffer,
        depth_buffer: &wgpu::Buffer,
        vertex_buffer: &wgpu::Buffer,
        uniform: &wgpu::Buffer,
    ) -> Self {
        // Bind Group for Color Buffer
        let output_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Output Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output_buffer.as_entire_binding(),
            }],
        });

        // Bind Group for Depth Buffer
        let depth_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Depth Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: depth_buffer.as_entire_binding(),
            }],
        });

        // Bind Group for Uniforms
        let uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Uniform Bind Group"),
            layout: &pipeline.get_bind_group_layout(2),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        // Bind Group for Vertex Buffer
        let vertex_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Vertex Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(3),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            }],
        });

        Self {
            output_buffer,
            depth_buffer,
            uniform,
            vertex_buffer,
        }
    }

    pub fn update_vertex_buffer(
        &mut self,
        device: &wgpu::Device,
        RasterPass { pipeline }: &RasterPass,
        vertex_buffer: &wgpu::Buffer,
    ) {
        self.vertex_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster: Vertex Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            }],
        });
    }
}

impl<'a> RasterPass {
    pub fn record<'pass>(
        &'a self,
        cpass: &mut wgpu::ComputePass<'pass>,
        bindings: &'a RasterBindings,
        dispatch_size: u32,
    ) where
        'a: 'pass,
    {
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bindings.output_buffer, &[]);
        cpass.set_bind_group(1, &bindings.depth_buffer, &[]);
        cpass.set_bind_group(2, &bindings.uniform, &[]);
        cpass.set_bind_group(3, &bindings.vertex_buffer, &[]);
        cpass.dispatch_workgroups(dispatch_size, 1, 1);
    }
}
