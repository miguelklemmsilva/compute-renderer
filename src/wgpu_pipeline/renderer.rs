use wgpu::util::DeviceExt;

use crate::{
    camera::CameraUniform,
    scene::Scene,
};

/// Data to hold GPU buffers and bind groups for each Model in the Scene.
pub struct ModelRenderData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

/// The main renderer that uses wgpu's standard raster pipeline.
/// This handles all the GPU resource management and rendering operations.
pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    // Pipeline
    pub render_pipeline: wgpu::RenderPipeline,

    // Depth buffer
    pub depth_texture_view: wgpu::TextureView,

    // Camera and effect buffers
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,

    // Bind group for camera & effect data
    pub global_bind_group: wgpu::BindGroup,

    // Scene geometry (one ModelRenderData per loaded model)
    pub model_data: Vec<ModelRenderData>,
}

impl WgpuRenderer {
    /// Create a new raster-based wgpu renderer.
    ///
    /// # Arguments
    /// * `instance` - The wgpu instance to use
    /// * `surface` - The surface to render to
    /// * `width` - The initial width of the surface
    /// * `height` - The initial height of the surface
    /// * `scene` - The scene to render
    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        scene: &Scene,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None, // Trace path
            )
            .await
            .expect("Failed to create device");

        // === 2) Create surface configuration
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .first()
            .cloned()
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        // === 3) Create depth texture
        let depth_texture = create_depth_texture(&device, &config, "depth_texture");
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // === 4) Create (camera + lights + effects) buffers & bind group
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut initial_lights = scene.lights.clone();
        if !scene.lights.is_empty() {
            initial_lights[..scene.lights.len()].copy_from_slice(&scene.lights);
        }
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&initial_lights),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let global_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global Bind Group Layout"),
                entries: &[
                    // Camera uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(
                                std::num::NonZeroU64::new(
                                    std::mem::size_of::<CameraUniform>() as u64
                                )
                                .unwrap(),
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global Bind Group"),
            layout: &global_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });

        // === 5) Create the render pipeline
        let shader_source = include_str!("shaders.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Raster Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&global_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create the pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // Counter-clockwise winding
                cull_mode: Some(wgpu::Face::Back), // Back-face culling
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: create_depth_texture_format(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // === 6) Create model buffers for each model in the scene
        let mut model_data = Vec::new();
        for model in &scene.models {
            // Create vertex buffer
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&model.processed_vertices_wgpu),
                usage: wgpu::BufferUsages::VERTEX,
            });

            // Create index buffer
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&model.processed_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            // The total index count
            let index_count = model.processed_indices.len() as u32;

            model_data.push(ModelRenderData {
                vertex_buffer,
                index_buffer,
                index_count,
            });
        }

        // Return the newly created WgpuRenderer
        Self {
            device,
            queue,
            config,
            render_pipeline,
            depth_texture_view,
            camera_buffer,
            light_buffer,
            global_bind_group,
            model_data,
        }
    }

    /// Resize the renderer's resources when the window size changes.
    ///
    /// # Arguments
    /// * `config` - The new surface configuration
    pub fn resize(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.config = config.clone();
        // Recreate depth texture with new size
        let depth_texture = create_depth_texture(&self.device, config, "depth_texture");
        self.depth_texture_view =
            depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Render the current scene.
    ///
    /// # Arguments
    /// * `surface` - The surface to render to
    /// * `scene` - The scene to render
    ///
    /// # Returns
    /// * `Result<(), wgpu::SurfaceError>` - Ok if rendering succeeded, Err if there was a surface error
    pub fn render(
        &mut self,
        surface: &wgpu::Surface<'_>,
        scene: &Scene,
    ) -> Result<(), wgpu::SurfaceError> {
        // Get the next frame
        let frame = surface.get_current_texture()?;

        // Create a view of the surface texture
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create a command encoder for recording commands
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Update camera uniform if there's an active camera
        if let Some(camera) = scene.get_active_camera() {
            let mut camera_uniform = CameraUniform::default();
            camera_uniform.update_view_proj(camera);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[camera_uniform]),
            );
        }

        // Update light uniforms if there are lights
        if !scene.lights.is_empty() {
            self.queue
                .write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&scene.lights));
        }

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set the pipeline
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.global_bind_group, &[]);

            // Draw each model
            for model_data in &self.model_data {
                render_pass.set_vertex_buffer(0, model_data.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(model_data.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model_data.index_count, 0, 0..1);
            }
        }

        // Submit command buffer and present frame
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}

fn create_depth_texture_format() -> wgpu::TextureFormat {
    wgpu::TextureFormat::Depth24Plus
}

fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: create_depth_texture_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    /// Returns the wgpu layout describing how the `Vertex` is laid out in memory
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: 12 as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: 24 as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
