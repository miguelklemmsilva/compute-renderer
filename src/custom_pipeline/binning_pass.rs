use wgpu::PipelineCompilationOptions;

use super::GpuBuffers;

pub struct BinningPass {
    pub pipeline_count: wgpu::ComputePipeline,
    pub pipeline_scan_first: wgpu::ComputePipeline,
    pub pipeline_scan_second: wgpu::ComputePipeline,
    pub pipeline_store: wgpu::ComputePipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
}

impl BinningPass {
    pub fn new(device: &wgpu::Device, buffers: &GpuBuffers) -> Self {
        // 1) Create bind group layouts
        let group0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Binning Pass: Group0 Layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
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
            label: Some("BinningPass::Group1"),
            entries: &[
                // [0] screen_dims
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
            label: Some("Binning Pipeline Layout"),
            bind_group_layouts: &[&group0_layout, &group1_layout],
            push_constant_ranges: &[],
        });

        // 2) Create shader module from WGSL
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Binning WGSL"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/binning.wgsl"
            ))),
        });

        // 3) Create compute pipelines
        let pipeline_count = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Count Triangles"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("count_triangles"),
            cache: None,
            compilation_options: PipelineCompilationOptions::default(),
        });

        let pipeline_scan_first =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Scan First Pass"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("scan_first_pass"),
                cache: None,
                compilation_options: PipelineCompilationOptions::default(),
            });

        let pipeline_scan_second =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Scan Second Pass"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("scan_second_pass"),
                cache: None,
                compilation_options: PipelineCompilationOptions::default(),
            });

        let pipeline_store = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Store Triangles"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("store_triangles"),
            cache: None,
            compilation_options: PipelineCompilationOptions::default(),
        });

        // 4) Create bind groups
        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binning Pass: Group0"),
            layout: &group0_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.projected_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.tile_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.triangle_list_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.partial_sums_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffers.triangle_meta_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BinningPass::BG1"),
            layout: &group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.screen_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline_count,
            pipeline_scan_first,
            pipeline_scan_second,
            pipeline_store,
            bind_group_0,
            bind_group_1,
        }
    }

    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_tris: u32,
        total_pixel_dispatch: u32,
    ) {
        let total_threads_needed = (total_tris as f32).ceil();

        let gx_tris = (total_threads_needed as f32).sqrt().ceil() as u32;
        let gy_tris = ((total_threads_needed as f32) / (gx_tris as f32)).ceil() as u32;

        // ---------------------------------------------------------------------
        // 1) count_triangles
        // ---------------------------------------------------------------------
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning::count_triangles"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_count);
            pass.set_bind_group(0, &self.bind_group_0, &[]);
            pass.set_bind_group(1, &self.bind_group_1, &[]);

            pass.dispatch_workgroups(gx_tris, gy_tris, 1);
        }

        // ---------------------------------------------------------------------
        // 2) scan_first_pass: do a local prefix-scan per tile
        //    Each tile is handled by exactly one thread => #threads = #tiles
        // ---------------------------------------------------------------------

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning::scan_first_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_scan_first);
            pass.set_bind_group(0, &self.bind_group_0, &[]);
            pass.set_bind_group(1, &self.bind_group_1, &[]);
            pass.dispatch_workgroups(total_pixel_dispatch, 1, 1);
        }

        // ---------------------------------------------------------------------
        // 3) scan_second_pass: each tile adds the partial sum from its WG
        // ---------------------------------------------------------------------
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning::scan_second_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_scan_second);
            pass.set_bind_group(0, &self.bind_group_0, &[]);
            pass.set_bind_group(1, &self.bind_group_1, &[]);
            pass.dispatch_workgroups(total_pixel_dispatch, 1, 1);
        }

        // ---------------------------------------------------------------------
        // 4) store_triangles: final pass that writes triangle indices
        // ---------------------------------------------------------------------
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning::store_triangles"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline_store);
            pass.set_bind_group(0, &self.bind_group_0, &[]);
            pass.set_bind_group(1, &self.bind_group_1, &[]);

            pass.dispatch_workgroups(gx_tris, gy_tris, 1);
        }
    }
}
