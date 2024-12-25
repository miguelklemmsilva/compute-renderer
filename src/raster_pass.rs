pub struct RasterPass {
    pub pipeline: wgpu::ComputePipeline,
}

impl RasterPass {
    pub fn new(device: &wgpu::Device) -> Self {
        // Combine output and depth buffers into one bind group layout
        let frame_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Frame Buffer Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Screen dimensions bind group layout
        let screen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Screen Bind Group Layout"),
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

        // Camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
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

        // Combine vertex and light buffers into one bind group layout
        let geometry_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Geometry Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Combine texture and texture info buffers into one bind group layout
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Effect bind group layout
        let effect_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Effect Bind Group Layout"),
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

        // Create Pipeline Layout with consolidated bind group layouts
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raster Pipeline Layout"),
            bind_group_layouts: &[
                &frame_buffer_bind_group_layout,
                &screen_bind_group_layout,
                &camera_bind_group_layout,
                &geometry_bind_group_layout,
                &texture_bind_group_layout,
                &effect_bind_group_layout,
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
    pub frame_buffer: wgpu::BindGroup,
    pub screen: wgpu::BindGroup,
    pub camera: wgpu::BindGroup,
    pub geometry: wgpu::BindGroup,
    pub texture: wgpu::BindGroup,
    pub effect: wgpu::BindGroup,
}

impl RasterBindings {
    pub fn new(
        device: &wgpu::Device,
        RasterPass { pipeline }: &RasterPass,
        output_buffer: &wgpu::Buffer,
        depth_buffer: &wgpu::Buffer,
        vertex_buffer: &wgpu::Buffer,
        texture_buffer: &wgpu::Buffer,
        texture_infos_buffer: &wgpu::Buffer,
        screen_uniform: &wgpu::Buffer,
        camera_buffer: &wgpu::Buffer,
        light_buffer: &wgpu::Buffer,
        effect_buffer: &wgpu::Buffer,
    ) -> Self {
        // Frame buffer bind group (color + depth)
        let frame_buffer = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame Buffer Bind Group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: depth_buffer.as_entire_binding(),
                },
            ],
        });

        // Screen dimensions bind group
        let screen = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Screen Bind Group"),
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_uniform.as_entire_binding(),
            }],
        });

        // Camera bind group
        let camera = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &pipeline.get_bind_group_layout(2),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Geometry bind group (vertices + lights)
        let geometry = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Geometry Bind Group"),
            layout: &pipeline.get_bind_group_layout(3),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        // Texture bind group (texture data + info)
        let texture = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &pipeline.get_bind_group_layout(4),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: texture_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture_infos_buffer.as_entire_binding(),
                },
            ],
        });

        // Effect bind group
        let effect = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Effect Bind Group"),
            layout: &pipeline.get_bind_group_layout(5),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: effect_buffer.as_entire_binding(),
            }],
        });

        Self {
            frame_buffer,
            screen,
            camera,
            geometry,
            texture,
            effect,
        }
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
        cpass.set_bind_group(0, &bindings.frame_buffer, &[]);
        cpass.set_bind_group(1, &bindings.screen, &[]);
        cpass.set_bind_group(2, &bindings.camera, &[]);
        cpass.set_bind_group(3, &bindings.geometry, &[]);
        cpass.set_bind_group(4, &bindings.texture, &[]);
        cpass.set_bind_group(5, &bindings.effect, &[]);
        cpass.dispatch_workgroups(dispatch_size, 1, 1);
    }
}
