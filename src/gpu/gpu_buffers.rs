use wgpu::util::DeviceExt;

use crate::{camera, effect::EffectUniform, scene, util::{TextureInfo, Uniform, Vertex}};

use super::{binning_pass::MAX_TRIANGLES_PER_TILE, raster_pass::TILE_SIZE};

pub struct GpuBuffers {
    // Buffers
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub effect_buffer: wgpu::Buffer,
    pub screen_buffer: wgpu::Buffer,

    pub vertex_buffer: wgpu::Buffer,
    pub projected_buffer: wgpu::Buffer,

    // For collecting all pixel coverage from raster stage
    pub fragment_buffer: wgpu::Buffer,

    pub output_buffer: wgpu::Buffer,

    // Textures
    pub texture_buffer: wgpu::Buffer,
    pub texture_info_buffer: wgpu::Buffer,

    // Tile buffer
    pub tile_buffer: wgpu::Buffer,
}

impl GpuBuffers {
    pub fn new(device: &wgpu::Device, width: usize, height: usize, scene: &scene::Scene) -> Self {
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
        let fragment_size_bytes = 40 /* or however large each Fragment is */;
        let fragment_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fragment Buffer"),
            size: max_fragments * fragment_size_bytes,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
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

        // 10) texture buffers
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
            contents: bytemuck::cast_slice(texture_infos.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let num_tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;

        let num_tiles = num_tiles_x * num_tiles_y;

        let tile_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tile Buffer"),
            size: (num_tiles * (std::mem::size_of::<u32>() * 2 + std::mem::size_of::<u32>() * MAX_TRIANGLES_PER_TILE as usize)) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            camera_buffer,
            light_buffer,
            effect_buffer,
            screen_buffer,
            vertex_buffer,
            projected_buffer,
            fragment_buffer,
            output_buffer,
            texture_buffer,
            texture_info_buffer,
            tile_buffer,
        }
    }
}