pub struct RasterPass {
    pub pipeline: wgpu::ComputePipeline,
}

impl RasterPass {
    // Helper functions for creating bind group layouts
    fn create_storage_bind_group_layout(
        device: &wgpu::Device,
        label: &str,
        read_only: bool,
        bindings: usize,
    ) -> wgpu::BindGroupLayout {
        let entries = (0..bindings)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding: binding as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect::<Vec<_>>();

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &entries,
        })
    }

    fn create_uniform_bind_group_layout(
        device: &wgpu::Device,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
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
        })
    }

    pub fn new(device: &wgpu::Device) -> Self {
        // Create bind group layouts using helper functions
        let frame_buffer_bind_group_layout = Self::create_storage_bind_group_layout(
            device,
            "Frame Buffer Bind Group Layout",
            false,
            2,
        );
        let screen_bind_group_layout =
            Self::create_uniform_bind_group_layout(device, "Screen Bind Group Layout");
        let camera_bind_group_layout =
            Self::create_uniform_bind_group_layout(device, "Camera Bind Group Layout");
        let geometry_bind_group_layout =
            Self::create_storage_bind_group_layout(device, "Geometry Bind Group Layout", true, 2);
        let texture_bind_group_layout =
            Self::create_storage_bind_group_layout(device, "Texture Bind Group Layout", true, 2);
        let effect_bind_group_layout =
            Self::create_uniform_bind_group_layout(device, "Effect Bind Group Layout");

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
    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        label: &str,
        resources: &[wgpu::BindingResource],
    ) -> wgpu::BindGroup {
        let entries: Vec<_> = resources
            .iter()
            .enumerate()
            .map(|(i, resource)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: resource.clone(),
            })
            .collect();

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &entries,
        })
    }

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
        let frame_buffer = Self::create_bind_group(
            device,
            &pipeline.get_bind_group_layout(0),
            "Frame Buffer Bind Group",
            &[
                output_buffer.as_entire_binding(),
                depth_buffer.as_entire_binding(),
            ],
        );

        // Screen dimensions bind group
        let screen = Self::create_bind_group(
            device,
            &pipeline.get_bind_group_layout(1),
            "Screen Bind Group",
            &[screen_uniform.as_entire_binding()],
        );

        // Camera bind group
        let camera = Self::create_bind_group(
            device,
            &pipeline.get_bind_group_layout(2),
            "Camera Bind Group",
            &[camera_buffer.as_entire_binding()],
        );

        // Geometry bind group (vertices + lights)
        let geometry = Self::create_bind_group(
            device,
            &pipeline.get_bind_group_layout(3),
            "Geometry Bind Group",
            &[
                vertex_buffer.as_entire_binding(),
                light_buffer.as_entire_binding(),
            ],
        );

        // Texture bind group (texture data + info)
        let texture = Self::create_bind_group(
            device,
            &pipeline.get_bind_group_layout(4),
            "Texture Bind Group",
            &[
                texture_buffer.as_entire_binding(),
                texture_infos_buffer.as_entire_binding(),
            ],
        );

        // Effect bind group
        let effect = Self::create_bind_group(
            device,
            &pipeline.get_bind_group_layout(5),
            "Effect Bind Group",
            &[effect_buffer.as_entire_binding()],
        );

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
