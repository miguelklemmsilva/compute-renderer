use wgpu::PipelineCompilationOptions;

use super::{
    util::create_buffer_bind_group_layout_entry,
    GpuBuffers,
};

pub struct BinningPass {
    pub pipeline_count: wgpu::ComputePipeline,
    pub pipeline_scan_first: wgpu::ComputePipeline,
    pub pipeline_scan_second: wgpu::ComputePipeline,
    pub pipeline_store: wgpu::ComputePipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
    pub bind_group_2: wgpu::BindGroup,
    pub bind_group_3: wgpu::BindGroup,
}

impl BinningPass {
    pub fn new(device: &wgpu::Device, buffers: &GpuBuffers) -> Self {
        // 1) Create bind group layouts
        let group0_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Binning Pass: Group0 Layout"),
            entries: &[
                create_buffer_bind_group_layout_entry(0, false),
                create_buffer_bind_group_layout_entry(1, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

        let group1_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BinningPass::Group1"),
            entries: &[create_buffer_bind_group_layout_entry(0, false)],
        });

        let group2_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BinningPass::Group2"),
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

        let group3_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BinningPass::Group3"),
            entries: &[create_buffer_bind_group_layout_entry(0, false)],
        });

        let count_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Count Pipeline layout"),
                bind_group_layouts: &[&group0_layout, &group1_layout, &group2_layout],
                push_constant_ranges: &[],
            });

        let scan_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scan Pipeline layout"),
            bind_group_layouts: &[&group0_layout, &group1_layout],
            push_constant_ranges: &[],
        });

        let store_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Store Pipeline layout"),
                bind_group_layouts: &[
                    &group0_layout,
                    &group1_layout,
                    &group2_layout,
                    &group3_layout,
                ],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/binning.wgsl"));

        let pipeline_count = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Count Triangles"),
            layout: Some(&count_pipeline_layout),
            module: &shader,
            entry_point: Some("count_triangles"),
            cache: None,
            compilation_options: PipelineCompilationOptions::default(),
        });

        let pipeline_scan_first =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Scan First Pass"),
                layout: Some(&scan_pipeline_layout),
                module: &shader,
                entry_point: Some("scan_first_pass"),
                cache: None,
                compilation_options: PipelineCompilationOptions::default(),
            });

        let pipeline_scan_second =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Scan Second Pass"),
                layout: Some(&scan_pipeline_layout),
                module: &shader,
                entry_point: Some("scan_second_pass"),
                cache: None,
                compilation_options: PipelineCompilationOptions::default(),
            });

        let pipeline_store = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Store Triangles"),
            layout: Some(&store_pipeline_layout),
            module: &shader,
            entry_point: Some("store_triangles"),
            cache: None,
            compilation_options: PipelineCompilationOptions::default(),
        });

        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binning Pass: Group0"),
            layout: &group0_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.tile_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.triangle_meta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.screen_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BinningPass::BG1"),
            layout: &group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.partial_sums_buffer.as_entire_binding(),
            }],
        });

        let bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BinningPass::BG2"),
            layout: &group2_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.projected_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_3 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BinningPass::BG3"),
            layout: &group3_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.triangle_list_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline_count,
            pipeline_scan_first,
            pipeline_scan_second,
            pipeline_store,
            bind_group_0,
            bind_group_1,
            bind_group_2,
            bind_group_3,
        }
    }

    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_tris: u32,
        total_pixel_dispatch: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Binning::count_triangles"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, &self.bind_group_0, &[]);
        pass.set_bind_group(1, &self.bind_group_1, &[]);
        pass.set_bind_group(2, &self.bind_group_2, &[]);
        pass.set_bind_group(3, &self.bind_group_3, &[]);

        let total_threads_needed = (total_tris as f32).ceil();

        let gx_tris = (total_threads_needed as f32).sqrt().ceil() as u32;
        let gy_tris = ((total_threads_needed as f32) / (gx_tris as f32)).ceil() as u32;

        pass.set_pipeline(&self.pipeline_count);
        pass.dispatch_workgroups(gx_tris, gy_tris, 1);

        pass.set_pipeline(&self.pipeline_scan_first);
        pass.dispatch_workgroups(total_pixel_dispatch, 1, 1);

        pass.set_pipeline(&self.pipeline_scan_second);
        pass.dispatch_workgroups(total_pixel_dispatch, 1, 1);

        pass.set_pipeline(&self.pipeline_store);
        pass.dispatch_workgroups(gx_tris, gy_tris, 1);
    }
}
