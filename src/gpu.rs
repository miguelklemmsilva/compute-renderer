use wgpu::util::DeviceExt;

use crate::{
    camera,
    clear_pass::ClearPass,
    effect::EffectUniform,
    raster_pass::{RasterBindings, RasterPass},
    scene,
    util::{dispatch_size, Uniform, Vertex},
};

pub struct GPU {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub effect_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,

    pub raster_pass: RasterPass,
    pub raster_bindings: RasterBindings,

    clear_pass: ClearPass,
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
    fn create_buffer<T: bytemuck::Pod>(
        device: &wgpu::Device,
        label: &str,
        contents: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(contents),
            usage,
        })
    }

    fn create_empty_buffer(
        device: &wgpu::Device,
        label: &str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    pub async fn new(width: usize, height: usize, scene: &scene::Scene) -> GPU {
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

        let raster_pass = RasterPass::new(&device);
        let clear_pass = ClearPass::new(&device);

        // Create uniform buffers
        let screen_uniform = Self::create_buffer(
            &device,
            "Screen Uniform Buffer",
            &[Uniform::new(width as _, height as _)],
            wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        );

        // Create vertex buffer
        let vertices = scene
            .models
            .iter()
            .flat_map(|model| model.vertices.clone())
            .collect::<Vec<Vertex>>();
        let vertex_buffer = Self::create_buffer(
            &device,
            "Vertex Buffer",
            &vertices,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        );

        // Create texture buffers
        let (texture_buffer, texture_infos_buffer) = {
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

            let fallback_data = vec![0xffffffffu32]; // White pixel
            let texture_data = if flattened_texture_data.is_empty() {
                &fallback_data
            } else {
                &flattened_texture_data
            };

            let texture_infos_data = if texture_infos.is_empty() {
                vec![TextureInfo {
                    offset: 0,
                    width: 1,
                    height: 1,
                    _padding: 0,
                }]
            } else {
                texture_infos
            };

            (
                Self::create_buffer(
                    &device,
                    "Texture Buffer",
                    texture_data,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                Self::create_buffer(
                    &device,
                    "Texture Info Buffer",
                    &texture_infos_data,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
        };

        // Create camera buffer
        let active_camera = scene.get_active_camera().expect("No active camera");
        let mut camera_uniform = camera::CameraUniform::default();
        camera_uniform.update_view_proj(&active_camera);
        let camera_buffer = Self::create_buffer(
            &device,
            "Camera Buffer",
            &[camera_uniform],
            wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        );

        // Create depth buffer
        let depth_buffer = Self::create_empty_buffer(
            &device,
            "Depth Buffer",
            (height * width * std::mem::size_of::<f32>()) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        // Create output buffer
        let output_buffer = Self::create_empty_buffer(
            &device,
            "Output Buffer",
            (width * height * std::mem::size_of::<u32>()) as u64,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::MAP_READ,
        );

        // Create light buffer
        let lights = scene.get_lights();
        let light_buffer = Self::create_buffer(
            &device,
            "Light Buffer",
            lights,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        // Create effect buffer
        let effect_uniform = EffectUniform::default();
        let effect_buffer = Self::create_buffer(
            &device,
            "Effect Buffer",
            &[effect_uniform],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let raster_bindings = RasterBindings::new(
            &device,
            &raster_pass,
            &output_buffer,
            &depth_buffer,
            &vertex_buffer,
            &texture_buffer,
            &texture_infos_buffer,
            &screen_uniform,
            &camera_buffer,
            &light_buffer,
            &effect_buffer,
        );

        GPU {
            device,
            queue,
            camera_buffer,
            light_buffer,
            effect_buffer,
            output_buffer,
            raster_pass,
            raster_bindings,
            clear_pass,
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
                label: Some("Main Encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Clear Pass"),
                timestamp_writes: None,
            });

            self.clear_pass.record(
                &mut cpass,
                &self.raster_bindings,
                dispatch_size((width * height) as u32),
            );
        }

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Raster Pass"),
                timestamp_writes: None,
            });

            let num_triangles = scene
                .models
                .iter()
                .map(|model| model.vertices.len() / 3)
                .sum::<usize>();

            self.raster_pass.record(
                &mut cpass,
                &self.raster_bindings,
                dispatch_size(num_triangles as u32),
            );
        }

        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.output_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        self.device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let buffer = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.output_buffer.unmap();

        buffer
    }
}
