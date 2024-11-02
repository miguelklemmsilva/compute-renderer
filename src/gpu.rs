use wgpu::util::DeviceExt;

use crate::{
    camera,
    clear_pass::ClearPass,
    raster_pass::{RasterBindings, RasterPass},
    scene,
    util::{self, dispatch_size, Uniform, Vertex},
};

pub struct GPU {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub camera_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,

    pub raster_pass: RasterPass,
    pub raster_bindings: RasterBindings,

    clear_pass: ClearPass,
}

impl GPU {
    pub async fn new(width: usize, height: usize, scene: &scene::Scene) -> GPU {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Failed to find an appropriate adapter");

        let device_info = adapter.get_info();
        println!("Backend: {:?}", device_info.backend);
        println!("Device Name: {}", device_info.name);
        println!("Device Type: {:?}", device_info.device_type);

        let limits = adapter.limits();
        let features = adapter.features();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: features,
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let raster_pass = RasterPass::new(&device);
        let clear_pass = ClearPass::new(&device);

        let screen_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Uniform Buffer"),
            contents: bytemuck::bytes_of(&Uniform::new(width as _, height as _)),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let vertices = scene
            .models
            .iter()
            .flat_map(|model| model.vertices.clone())
            .collect::<Vec<Vertex>>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let img = image::open("assets/african_head_diffuse.tga")
            .unwrap()
            .to_rgba8();
        let (tex_width, tex_height) = img.dimensions();
        let texture_data = img.into_raw();

        // Pack the RGBA data into u32 values
        let mut packed_texture_data = Vec::with_capacity((tex_width * tex_height) as usize);
        for i in 0..(texture_data.len() / 4) {
            let r = texture_data[i * 4 + 0] as u32;
            let g = texture_data[i * 4 + 1] as u32;
            let b = texture_data[i * 4 + 2] as u32;
            let a = texture_data[i * 4 + 3] as u32;
            let packed = (r << 24) | (g << 16) | (b << 8) | a;
            packed_texture_data.push(packed);
        }

        println!("Texture data sample: {:?}", &packed_texture_data[..10]);

        // Create a buffer for the texture data
        let texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: bytemuck::cast_slice(&packed_texture_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let texture_dims = util::TextureDims {
            width: tex_width,
            height: tex_height,
        };

        let texture_dims_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Dims Buffer"),
            contents: bytemuck::bytes_of(&texture_dims),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // for vertex in &vertices {
        //     println!("UV: {:?}", vertex.tex_coords);
        // }

        let active_camera = scene.get_active_camera().expect("No active camera");
        let mut camera_uniform = camera::CameraUniform::default();
        camera_uniform.update_view_proj(&active_camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });

        let depth_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Depth Buffer"),
            size: (height as usize * width as usize * std::mem::size_of::<f32>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (width * height * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let raster_bindings = RasterBindings::new(
            &device,
            &raster_pass,
            &output_buffer,
            &depth_buffer,
            &vertex_buffer,
            &texture_buffer,
            &texture_dims_buffer,
            &screen_uniform,
            &camera_buffer,
        );

        GPU {
            device,
            queue,

            camera_buffer,
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
