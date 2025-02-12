use wgpu::util::DeviceExt;

use crate::{
    camera,
    custom_pipeline::util::{Fragment, MaterialInfo, Uniform},
    effect::EffectUniform,
    scene, vertex::GpuVertex,
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

        // 2) Get pre-processed data from all models
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut all_texture_data = Vec::new();
        let mut material_infos = Vec::new();

        for model in &scene.models {
            // Add pre-processed vertices and indices
            vertices.extend_from_slice(&model.processed_vertices_custom);
            indices.extend_from_slice(&model.processed_indices);
            material_infos.extend_from_slice(&model.processed_materials);
            all_texture_data.extend_from_slice(&model.processed_textures);
        }

        // If no textures exist, use a small fallback
        let texture_data = if all_texture_data.is_empty() {
            vec![0]
        } else {
            all_texture_data
        };

        let material_data = if material_infos.is_empty() {
            vec![MaterialInfo::default()]
        } else {
            material_infos
        };

        let index_length = indices.len();

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
            size: (vertices.len() * std::mem::size_of::<GpuVertex>()) as u64,
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

        let texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: bytemuck::cast_slice(&texture_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 3) Create the texture info buffer
        let material_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&material_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 10) tile buffer - now just stores count and offset per tile
        let num_tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles = num_tiles_x * num_tiles_y;

        let total_triangles = (index_length / 3) as u32;

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
                128,             // Minimum allocation to handle dense areas
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
            texture_info_buffer: material_info_buffer,
            tile_buffer,
            triangle_list_buffer,
            partial_sums_buffer,
        }
    }
}
