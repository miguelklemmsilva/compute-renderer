use wgpu::util::DeviceExt;

use crate::{
    camera,
    clear_pass::ClearPass,
    effect::EffectUniform,
    fragment_pass::FragmentPass,
    raster_pass::RasterPass,
    scene,
    util::{dispatch_size, Uniform, Vertex},
    vertex_pass::VertexPass, // the new passes:
};

pub struct GPU {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    // Buffers
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub effect_buffer: wgpu::Buffer,
    pub screen_buffer: wgpu::Buffer,

    pub vertex_buffer: wgpu::Buffer,
    pub projected_buffer: wgpu::Buffer,

    // For collecting all pixel coverage from raster stage
    pub fragment_buffer: wgpu::Buffer,
    pub fragment_count: wgpu::Buffer,

    pub output_buffer: wgpu::Buffer,
    pub depth_buffer: wgpu::Buffer,

    // Textures
    pub texture_buffer: wgpu::Buffer,
    pub texture_info_buffer: wgpu::Buffer,

    // Our 4 passes
    pub clear_pass: ClearPass,
    pub vertex_pass: VertexPass,
    pub raster_pass: RasterPass,
    pub fragment_pass: FragmentPass,

    // Example bind groups for each pass
    pub clear_bind_group_0: wgpu::BindGroup,
    pub clear_bind_group_1: wgpu::BindGroup,

    pub vertex_bind_group_0: wgpu::BindGroup,
    pub vertex_bind_group_1: wgpu::BindGroup,
    pub vertex_bind_group_2: wgpu::BindGroup,
    pub vertex_bind_group_3: wgpu::BindGroup,

    pub raster_bind_group_0: wgpu::BindGroup,
    pub raster_bind_group_1: wgpu::BindGroup,

    pub fragment_bind_group_0: wgpu::BindGroup,
    pub fragment_bind_group_1: wgpu::BindGroup,
    pub fragment_bind_group_2: wgpu::BindGroup,
    pub fragment_bind_group_3: wgpu::BindGroup,
    pub fragment_bind_group_4: wgpu::BindGroup,
    pub fragment_bind_group_5: wgpu::BindGroup,
    pub fragment_bind_group_6: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct TextureInfo {
    offset: u32,
    width: u32,
    height: u32,
    _padding: u32,
}

impl GPU {
    pub async fn new(width: usize, height: usize, scene: &scene::Scene) -> Self {
        // Init instance/adapter/device
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Failed to find an appropriate adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: adapter.features(),
                    required_limits: adapter.limits(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Create the four pipelines
        let clear_pass = ClearPass::new(&device);
        let vertex_pass = VertexPass::new(&device);
        let raster_pass = RasterPass::new(&device);
        let fragment_pass = FragmentPass::new(&device);

        // ------------------------------------------------------------
        // Create Buffers
        // ------------------------------------------------------------
        // 1) screen buffer
        let screen_uniform_data = Uniform::new(width as f32, height as f32);
        let screen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Buffer"),
            contents: bytemuck::bytes_of(&screen_uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2) vertex buffer
        let vertices: Vec<Vertex> = scene
            .models
            .iter()
            .flat_map(|model| model.vertices.clone())
            .collect();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 3) projected buffer (same size as vertex_buffer)
        let projected_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Projected Buffer"),
            size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // 4) fragment buffer (for worst-case, you decide the size)
        let max_fragments = (width * height) as u64; // example
                                                     // For the actual struct size, replicate your "Fragment" struct if needed
                                                     // This is just a placeholder:
        let fragment_size_bytes = 48 /* or however large each Fragment is */;
        let fragment_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fragment Buffer"),
            size: max_fragments * fragment_size_bytes,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // 5) fragment counter (u32 atomic)
        let fragment_count = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fragment Counter"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 6) camera buffer
        let active_camera = scene.get_active_camera().expect("No active camera");
        let mut camera_uniform = camera::CameraUniform::default();
        camera_uniform.update_view_proj(&active_camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 7) effect buffer
        let effect_data = EffectUniform::default();
        let effect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Effect Buffer"),
            contents: bytemuck::bytes_of(&effect_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 8) light buffer
        let lights = scene.get_lights();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&lights),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 9) output buffer
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (width * height * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // 10) depth buffer
        let depth_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Depth Buffer"),
            size: (width * height * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 11) texture buffers
        let mut flattened_texture_data = Vec::new();
        let mut texture_infos = Vec::new();
        for material in &scene.materials {
            let offset = flattened_texture_data.len() as u32;
            flattened_texture_data.extend_from_slice(&material.texture.data);
            texture_infos.push(TextureInfo {
                offset,
                width: material.texture.width,
                height: material.texture.height,
                _padding: 0,
            });
        }
        let fallback_data = vec![0xffffffffu32];
        let texture_data = if flattened_texture_data.is_empty() {
            &fallback_data
        } else {
            &flattened_texture_data
        };
        let texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: bytemuck::cast_slice(texture_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        if texture_infos.is_empty() {
            texture_infos.push(TextureInfo {
                offset: 0,
                width: 1,
                height: 1,
                _padding: 0,
            });
        }
        let texture_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Info Buffer"),
            contents: bytemuck::cast_slice(&texture_infos),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // ------------------------------------------------------------
        // Create Bind Groups
        // ------------------------------------------------------------

        // CLEAR PASS
        // group(0) -> (output_buffer, depth_buffer, fragment_counter, fragment_buffer)
        let clear_group0_layout = clear_pass.pipeline.get_bind_group_layout(0);
        let clear_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Clear: Frame Buffer Bind Group"),
            layout: &clear_group0_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: depth_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fragment_count.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: fragment_buffer.as_entire_binding(),
                },
            ],
        });

        // group(1) -> screen
        let clear_group1_layout = clear_pass.pipeline.get_bind_group_layout(1);
        let clear_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Clear: Screen Bind Group"),
            layout: &clear_group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });

        // VERTEX PASS
        // group(0) -> (vertex_buffer, projected_buffer)
        let vertex_group0_layout = vertex_pass.pipeline.get_bind_group_layout(0);
        let vertex_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Pass: Group0"),
            layout: &vertex_group0_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: projected_buffer.as_entire_binding(),
                },
            ],
        });
        // group(1) -> screen
        let vertex_group1_layout = vertex_pass.pipeline.get_bind_group_layout(1);
        let vertex_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Pass: Group1 (Screen)"),
            layout: &vertex_group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });
        // group(2) -> camera
        let vertex_group2_layout = vertex_pass.pipeline.get_bind_group_layout(2);
        let vertex_bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Pass: Group2 (Camera)"),
            layout: &vertex_group2_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        // group(3) -> effect
        let vertex_group3_layout = vertex_pass.pipeline.get_bind_group_layout(3);
        let vertex_bind_group_3 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Pass: Group3 (Effect)"),
            layout: &vertex_group3_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: effect_buffer.as_entire_binding(),
            }],
        });

        // RASTER PASS
        // group(0) -> (projected_buffer, fragment_buffer, fragment_count)
        let raster_group0_layout = raster_pass.pipeline.get_bind_group_layout(0);
        let raster_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster Pass: Group0"),
            layout: &raster_group0_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: projected_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fragment_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fragment_count.as_entire_binding(),
                },
            ],
        });
        // group(1) -> screen
        let raster_group1_layout = raster_pass.pipeline.get_bind_group_layout(1);
        let raster_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raster Pass: Group1 (Screen)"),
            layout: &raster_group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });

        // FRAGMENT PASS
        // group(0) -> (output_buffer, depth_buffer)
        let fragment_group0_layout = fragment_pass.pipeline.get_bind_group_layout(0);
        let fragment_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group0"),
            layout: &fragment_group0_layout,
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
        // group(1) -> screen
        let fragment_group1_layout = fragment_pass.pipeline.get_bind_group_layout(1);
        let fragment_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group1"),
            layout: &fragment_group1_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });
        // group(2) -> camera
        let fragment_group2_layout = fragment_pass.pipeline.get_bind_group_layout(2);
        let fragment_bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group2"),
            layout: &fragment_group2_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        // group(3) -> light
        let fragment_group3_layout = fragment_pass.pipeline.get_bind_group_layout(3);
        let fragment_bind_group_3 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group3 (Lights)"),
            layout: &fragment_group3_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });
        // group(4) -> texture buffer + texture info
        let fragment_group4_layout = fragment_pass.pipeline.get_bind_group_layout(4);
        let fragment_bind_group_4 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group4 (Textures)"),
            layout: &fragment_group4_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: texture_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture_info_buffer.as_entire_binding(),
                },
            ],
        });
        // group(5) -> effect
        let fragment_group5_layout = fragment_pass.pipeline.get_bind_group_layout(5);
        let fragment_bind_group_5 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group5 (Effect)"),
            layout: &fragment_group5_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: effect_buffer.as_entire_binding(),
            }],
        });
        // group(6) -> fragment buffer + fragment count
        let fragment_group6_layout = fragment_pass.pipeline.get_bind_group_layout(6);
        let fragment_bind_group_6 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment Pass: Group6 (Fragments)"),
            layout: &fragment_group6_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: fragment_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fragment_count.as_entire_binding(),
                },
            ],
        });

        Self {
            device,
            queue,

            camera_buffer,
            light_buffer,
            effect_buffer,
            screen_buffer,

            vertex_buffer,
            projected_buffer,
            fragment_buffer,
            fragment_count,

            output_buffer,
            depth_buffer,

            texture_buffer,
            texture_info_buffer,

            clear_pass,
            vertex_pass,
            raster_pass,
            fragment_pass,

            clear_bind_group_0,
            clear_bind_group_1,

            vertex_bind_group_0,
            vertex_bind_group_1,
            vertex_bind_group_2,
            vertex_bind_group_3,

            raster_bind_group_0,
            raster_bind_group_1,

            fragment_bind_group_0,
            fragment_bind_group_1,
            fragment_bind_group_2,
            fragment_bind_group_3,
            fragment_bind_group_4,
            fragment_bind_group_5,
            fragment_bind_group_6,
        }
    }

    pub async fn execute_pipeline(
        &mut self,
        width: usize,
        height: usize,
        scene: &scene::Scene,
    ) -> Vec<u32> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // 1) CLEAR pass
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Clear Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.clear_pass.pipeline);
            // set groups
            cpass.set_bind_group(0, &self.clear_bind_group_0, &[]);
            cpass.set_bind_group(1, &self.clear_bind_group_1, &[]);
            let total_threads = (width * height) as u32;

            cpass.dispatch_workgroups(dispatch_size(total_threads), 1, 1);
        }

        // 2) VERTEX pass
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Vertex Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.vertex_pass.pipeline);
            cpass.set_bind_group(0, &self.vertex_bind_group_0, &[]);
            cpass.set_bind_group(1, &self.vertex_bind_group_1, &[]);
            cpass.set_bind_group(2, &self.vertex_bind_group_2, &[]);
            cpass.set_bind_group(3, &self.vertex_bind_group_3, &[]);
            let total_vertices =
                scene.models.iter().map(|m| m.vertices.len()).sum::<usize>() as u32;
            cpass.dispatch_workgroups(dispatch_size(total_vertices), 1, 1);
        }

        // 3) RASTER pass
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Raster Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.raster_pass.pipeline);
            cpass.set_bind_group(0, &self.raster_bind_group_0, &[]);
            cpass.set_bind_group(1, &self.raster_bind_group_1, &[]);
            let total_triangles = scene
                .models
                .iter()
                .map(|m| m.vertices.len() / 3)
                .sum::<usize>() as u32;
            let workgroups = dispatch_size(total_triangles);
            cpass.dispatch_workgroups(workgroups, 1, 1);
        }

        // 4) FRAGMENT pass
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fragment Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.fragment_pass.pipeline);
            cpass.set_bind_group(0, &self.fragment_bind_group_0, &[]);
            cpass.set_bind_group(1, &self.fragment_bind_group_1, &[]);
            cpass.set_bind_group(2, &self.fragment_bind_group_2, &[]);
            cpass.set_bind_group(3, &self.fragment_bind_group_3, &[]);
            cpass.set_bind_group(4, &self.fragment_bind_group_4, &[]);
            cpass.set_bind_group(5, &self.fragment_bind_group_5, &[]);
            cpass.set_bind_group(6, &self.fragment_bind_group_6, &[]);

            let max_fragments = (width * height) as u32;

            let workgroups = dispatch_size(max_fragments);

            cpass.dispatch_workgroups(workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Now map/read the output buffer as usual
        let buffer_slice = self.output_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let pixels = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.output_buffer.unmap();

        pixels
    }
}
