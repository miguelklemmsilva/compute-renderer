use wgpu::util::DeviceExt;

use crate::{
    camera,
    effect::EffectUniform,
    gpu::util::{Fragment, TextureInfo, Uniform, Vertex},
    scene,
};

use super::raster_pass::TILE_SIZE;

pub struct GpuBuffers {
    // Buffers
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub effect_buffer: wgpu::Buffer,
    pub screen_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub projected_buffer: wgpu::Buffer,
    pub fragment_buffer: wgpu::Buffer,
    pub output_buffer: wgpu::Buffer,
    pub texture_buffer: wgpu::Buffer,
    pub texture_info_buffer: wgpu::Buffer,
    pub tile_buffer: wgpu::Buffer,
    pub triangle_list_buffer: wgpu::Buffer,
    pub partial_sums_buffer: wgpu::Buffer,
}

impl GpuBuffers {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, scene: &scene::Scene) -> Self {
        // 1) screen buffer
        let screen_uniform_data = Uniform::new(width as f32, height as f32);
        let screen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Buffer"),
            contents: bytemuck::bytes_of(&screen_uniform_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2) vertex and index buffers
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0u32;

        for model in &scene.models {
            vertices.extend_from_slice(&model.vertices);
            // Adjust indices based on the current vertex offset
            indices.extend_from_slice(&model.indices.iter().map(|i| i.0 + index_offset).collect::<Vec<u32>>());
            index_offset += model.vertices.len() as u32;
        }

        let vertex_length = vertices.len();
        let index_length = indices.len();

        println!("Vertex length: {}", vertex_length);
        println!("Index length: {}", index_length);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 3) projected buffer (same size as vertex_buffer)
        let projected_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Projected Buffer"),
            size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // 4) fragment buffer
        let max_fragments = (width * height) as u64;
        let fragment_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fragment Buffer"),
            size: max_fragments * std::mem::size_of::<Fragment>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 5) camera buffer
        let active_camera = scene.get_active_camera().expect("No active camera");
        let mut camera_uniform = camera::CameraUniform::default();
        camera_uniform.update_view_proj(&active_camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 6) effect buffer
        let effect_data = EffectUniform::default();
        let effect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Effect Buffer"),
            contents: bytemuck::bytes_of(&effect_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 7) light buffer
        let lights = scene.get_lights();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&lights),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 8) output buffer
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (width as usize * height as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 9) texture buffers
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

        // 10) tile buffer - now just stores count and offset per tile
        let num_tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles = num_tiles_x * num_tiles_y;

        // Calculate total triangles and space per tile
        let total_triangles =
            (vertex_buffer.size() / std::mem::size_of::<Vertex>() as u64) as u32 / 3;

        // Calculate max triangles per tile based on screen coverage
        let avg_triangle_area = (width * height) as f32 / total_triangles as f32;
        let tile_area = (TILE_SIZE * TILE_SIZE) as f32;

        // Base estimate: how many triangles could fit in a tile
        let base_triangles_per_tile = (tile_area / avg_triangle_area * 2.0) as u32;

        // Add safety margin for overlapping triangles and uneven distribution
        let max_triangles_per_tile = std::cmp::max(
            base_triangles_per_tile,
            std::cmp::min(
                total_triangles, // Don't exceed total triangles
                64,              // Minimum allocation to handle dense areas
            ),
        );

        // Create tile buffer with count and offset for each tile
        let tile_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tile Buffer"),
            size: (num_tiles as usize * std::mem::size_of::<[u32; 4]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Size triangle list buffer based on max triangles per tile
        let triangle_list_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Triangle List Buffer"),
            size: (num_tiles as u64)
                * (max_triangles_per_tile as u64)
                * (std::mem::size_of::<u64>() as u64),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Calculate number of workgroups needed for parallel scan
        let num_tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;
        let total_workgroups = num_tiles_x * num_tiles_y;

        // Create partial sums buffer for parallel scan
        let partial_sums_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial Sums Buffer"),
            size: (total_workgroups as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            camera_buffer,
            light_buffer,
            effect_buffer,
            screen_buffer,
            vertex_buffer,
            index_buffer,
            projected_buffer,
            fragment_buffer,
            output_buffer,
            texture_buffer,
            texture_info_buffer,
            tile_buffer,
            triangle_list_buffer,
            partial_sums_buffer,
        }
    }
}
