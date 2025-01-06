pub struct RasterPass {
    pub pipeline: wgpu::ComputePipeline,
}

impl RasterPass {
    pub fn new(device: &wgpu::Device) -> Self {
        // Similar approach, we define layouts for:
        //   - group(0): projected buffer (read-only), fragment buffer (write-only), fragment counter (atomic)
        //   - group(1): screen uniform
        //
        // Adjust as needed for your actual usage.

        let group0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raster Pass: Group0 Layout"),
            entries: &[
                // binding 0 -> ProjectedVertexBuffer (read-only)
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
                // binding 1 -> FragmentBuffer (write-only)
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

        let group1_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raster Pass: Group1 Layout (Screen)"),
            entries: &[
                // screen dims
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raster Pipeline Layout"),
            bind_group_layouts: &[
                &group0_layout,
                &group1_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/rasteriser.wgsl"));

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Raster Pass Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("raster_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}