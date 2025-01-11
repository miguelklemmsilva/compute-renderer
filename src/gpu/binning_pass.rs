use wgpu::BindingResource;

use super::{raster_pass::TILE_SIZE, util::dispatch_size, GpuBuffers};
use crate::scene;

pub struct BinningPass {
    pub pipeline_count: wgpu::ComputePipeline,
    pub pipeline_store: wgpu::ComputePipeline,
    pub pipeline_scan_first: wgpu::ComputePipeline,
    pub pipeline_scan_second: wgpu::ComputePipeline,
    pub bind_group_0: wgpu::BindGroup,
    pub bind_group_1: wgpu::BindGroup,
}

impl BinningPass {
    pub fn new(device: &wgpu::Device, buffers: &GpuBuffers) -> Self {
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
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
            ],
        });

        let group1_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Binning Pass: Group1 Layout (Screen)"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Binning Pipeline Layout"),
            bind_group_layouts: &[&group0_layout, &group1_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/binning.wgsl"));

        let pipeline_count = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binning Pipeline - Count"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("count_triangles"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_store = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binning Pipeline - Store"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("store_triangles"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_scan_first =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Binning Pipeline - Scan First Pass"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("scan_first_pass"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let pipeline_scan_second =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Binning Pipeline - Scan Second Pass"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("scan_second_pass"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

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
                    resource: buffers.tile_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.triangle_list_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.partial_sums_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binning Pass: Group1"),
            layout: &group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.screen_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline_count,
            pipeline_store,
            pipeline_scan_first,
            pipeline_scan_second,
            bind_group_0,
            bind_group_1,
        }
    }

    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &scene::Scene,
        width: u32,
        height: u32,
    ) {
        // First pass: Count triangles per tile
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Binning Pass - Count"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline_count);
        cpass.set_bind_group(0, &self.bind_group_0, &[]);
        cpass.set_bind_group(1, &self.bind_group_1, &[]);

        // Calculate total number of triangles
        let total_triangles = scene
            .models
            .iter()
            .map(|m| m.vertices.len() / 3)
            .sum::<usize>() as u32;

        let workgroup_size = 16u32;
        let total_threads_needed =
            ((total_triangles as f32) / (workgroup_size * workgroup_size) as f32).ceil() as u32;

        let dispatch_x = (total_threads_needed as f32).sqrt().ceil() as u32;
        let dispatch_y = ((total_threads_needed as f32) / (dispatch_x as f32)).ceil() as u32;

        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        drop(cpass);

        // Parallel scan first pass
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Binning Pass - Scan First Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline_scan_first);
        cpass.set_bind_group(0, &self.bind_group_0, &[]);
        cpass.set_bind_group(1, &self.bind_group_1, &[]);

        let num_tiles_x = (width + TILE_SIZE as u32 - 1) / TILE_SIZE as u32;
        let num_tiles_y = (height + TILE_SIZE as u32 - 1) / TILE_SIZE as u32;
        let dispatch_x = (num_tiles_x + 31) / 32;
        let dispatch_y = (num_tiles_y + 31) / 32;

        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        drop(cpass);

        // Parallel scan second pass
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Binning Pass - Scan Second Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline_scan_second);
        cpass.set_bind_group(0, &self.bind_group_0, &[]);
        cpass.set_bind_group(1, &self.bind_group_1, &[]);

        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        drop(cpass);

        // Second pass: Store triangle indices
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Binning Pass - Store"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline_store);
        cpass.set_bind_group(0, &self.bind_group_0, &[]);
        cpass.set_bind_group(1, &self.bind_group_1, &[]);

        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }

    pub fn rebind(
        &mut self,
        device: &wgpu::Device,
        buffers: &GpuBuffers,
        triangle_list_buffer: BindingResource,
    ) {
        self.bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binning Pass: Group0"),
            layout: &self.pipeline_count.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.projected_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.tile_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: triangle_list_buffer,
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.partial_sums_buffer.as_entire_binding(),
                },
            ],
        });
    }
}
