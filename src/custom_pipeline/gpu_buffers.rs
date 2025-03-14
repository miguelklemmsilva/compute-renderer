use wgpu::util::DeviceExt;

use crate::{
    camera,
    custom_pipeline::util::{Fragment, ScreenUniform},
    effect::EffectUniform,
    scene,
};

use super::raster_pass::TILE_SIZE;

pub struct GpuBuffers {
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub effect_buffer: wgpu::Buffer,
    pub screen_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub projected_buffer: wgpu::Buffer,
    pub fragment_buffer: wgpu::Buffer,
    pub tile_buffer: wgpu::Buffer,
    pub triangle_list_buffer: wgpu::Buffer,
    pub partial_sums_buffer: wgpu::Buffer,
    pub triangle_meta_buffer: wgpu::Buffer,
    pub output_view: wgpu::TextureView,
}

impl GpuBuffers {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, scene: &scene::Scene) -> Self {
        let screen_uniform_data = ScreenUniform::new(width as f32, height as f32);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for model in &scene.models {
            // Add pre-processed vertices and indices
            vertices.extend_from_slice(&model.processed_vertices_custom);
            indices.extend_from_slice(&model.processed_indices);
        }

        let index_length = indices.len();

        let max_fragments = (width * height) as u64;

        let camera_uniform = camera::CameraUniform::default();

        let effect_data = EffectUniform::default();

        let num_tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles = (num_tiles_x * num_tiles_y) as u64;

        let total_triangles = (index_length / 3) as u32;

        // Calculate max triangles per tile based on screen coverage
        let avg_triangle_area = (width * height) as f32 / total_triangles as f32;
        let tile_area = (TILE_SIZE * TILE_SIZE) as f32;

        // Base estimate: how many triangles could fit in a tile
        let base_triangles_per_tile = (tile_area / avg_triangle_area * 2.0) as u32;

        // Add safety margin for overlapping triangles and uneven distribution
        let max_triangles_per_tile = std::cmp::max(base_triangles_per_tile, 128) as u64;

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct TriangleMeta {
            min_max: [f32; 4],
            start_tile: [u32; 2],
            tile_range: [u32; 2],
        }

        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Output Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
        };
        let output_texture = device.create_texture(&texture_desc);

        Self {
            camera_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::bytes_of(&camera_uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
            light_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Light Buffer"),
                contents: bytemuck::cast_slice(&scene.lights),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
            effect_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Effect Buffer"),
                contents: bytemuck::bytes_of(&effect_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
            screen_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Screen Buffer"),
                contents: bytemuck::bytes_of(&screen_uniform_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
            vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
            projected_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Projected Buffer"),
                size: (vertices.len() * std::mem::size_of::<[u32; 16]>()) as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            fragment_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Fragment Buffer"),
                size: max_fragments * std::mem::size_of::<Fragment>() as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            tile_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Tile Buffer"),
                size: num_tiles * std::mem::size_of::<[u32; 4]>() as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            triangle_list_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Triangle List Buffer"),
                size: num_tiles * max_triangles_per_tile * (std::mem::size_of::<u64>() as u64),
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            partial_sums_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Partial Sums Buffer"),
                size: num_tiles * std::mem::size_of::<u32>() as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            triangle_meta_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Triangle Meta Buffer"),
                size: num_tiles
                    * max_triangles_per_tile
                    * std::mem::size_of::<TriangleMeta>() as u64,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            output_view: output_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }
}
