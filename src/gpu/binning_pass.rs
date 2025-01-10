use super::GpuBuffers;
use crate::{scene, util::dispatch_size};

pub const MAX_TRIANGLES_PER_TILE: u32 = 1024;

pub struct BinningPass {
    pub pipeline: wgpu::ComputePipeline,
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

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binning Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("bin_triangles"),
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
            pipeline,
            bind_group_0,
            bind_group_1,
        }
    }

    pub fn execute(&self, encoder: &mut wgpu::CommandEncoder, scene: &scene::Scene) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Binning Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline);
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

        // Decide how to split total_threads_needed between X and Y dimensions.
        // For a near-square distribution:
        let dispatch_x = (total_threads_needed as f32).sqrt().ceil() as u32;
        let dispatch_y = ((total_threads_needed as f32) / (dispatch_x as f32)).ceil() as u32;

        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
