
// Simple wrapper for your Vertex pass pipeline.
pub struct VertexPass {
    pub pipeline: wgpu::ComputePipeline,
}

impl VertexPass {
    pub fn new(device: &wgpu::Device) -> Self {
        // Create Pipeline Layout. In this example we assume:
        //   - group(0): { read-only vertex buffer, write-only projected buffer }
        //   - group(1): { screen uniform }
        //   - group(2): { camera uniform }
        //   - group(3): { effect uniform }
        //
        // Adjust as needed to match your actual bind group usage.

        let group0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Pass: Group0 Layout"),
            entries: &[
                // binding 0 -> VertexBuffer (read-only storage)
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
                // binding 1 -> ProjectedVertexBuffer (write-only storage)
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
            label: Some("Vertex Pass: Group1 Layout (Screen Uniform)"),
            entries: &[
                // binding 0 -> Screen uniform
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

        let group2_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Pass: Group2 Layout (Camera)"),
            entries: &[
                // binding 0 -> Camera uniform
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

        let group3_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Pass: Group3 Layout (Effect)"),
            entries: &[
                // binding 0 -> Effect uniform
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

        // Pipeline Layout using the 4 group layouts
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Vertex Pipeline Layout"),
            bind_group_layouts: &[
                &group0_layout,
                &group1_layout,
                &group2_layout,
                &group3_layout,
            ],
            push_constant_ranges: &[],
        });

        // Load the WGSL code for vertex stage
        // (the file "shaders/vertex.wgsl" should define `@compute @workgroup_size(...) fn vertex_main() {}`)
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/vertex.wgsl"));

        // Create the compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Vertex Pass Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self { pipeline }
    }
}