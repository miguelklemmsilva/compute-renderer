use wgpu::util::DeviceExt;

use crate::{
    camera,
    effect::EffectUniform,
    gpu::util::{Fragment, Index, TextureInfo, Uniform, Vertex},
    scene,
};

use super::{raster_pass::TILE_SIZE, util::MaterialInfo};

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
        let mut all_texture_data = Vec::new();
        let mut material_infos = Vec::new();

        // Keep track of how many vertices we have so far
        // so that indices for subsequent meshes can be offset correctly.
        let mut current_vertex_count = 0;

        println!("now building buffers");

        // Loop **once** over every model in the scene
        for model in &scene.models {
            // 1) For each mesh in the model, gather vertices/indices
            for mesh in &model.meshes {
                // Append them to the big CPU-side list
                vertices.extend(mesh.vertices.clone());

                // Adjust the mesh’s indices so that they point to the correct vertex offset
                let converted_indices =
                    mesh.indices.iter().map(|Index(i)| i + current_vertex_count);
                indices.extend(converted_indices);

                // Now that we’ve appended these mesh vertices, increment the global offset
                current_vertex_count = vertices.len() as u32;
            }

            // 2) For each material, gather texture data and build your MaterialInfo
            for material in &model.materials {
                const NO_TEXTURE_INDEX: u32 = 0xFFFFFFFF;
                let texture_info = if let Some(tex) = &material.diffuse_texture {
                    let offset = all_texture_data.len() as u32;
                    all_texture_data.extend_from_slice(&tex.data);

                    TextureInfo {
                        offset,
                        width: tex.width,
                        height: tex.height,
                        _padding: 0,
                    }
                } else {
                    println!("no texture");
                    // “No texture” sentinel
                    TextureInfo {
                        offset: NO_TEXTURE_INDEX,
                        width: 0,
                        height: 0,
                        _padding: 0,
                    }
                };

                // Build a MaterialInfo for the material
                let material_info = MaterialInfo {
                    texture_info,
                    ambient: material.ambient,
                    _padding1: 0.0,
                    specular: material.specular,
                    _padding2: 0.0,
                    diffuse: material.diffuse_color,
                    shininess: material.shininess,
                    dissolve: material.dissolve,
                    optical_density: material.optical_density,
                    _padding3: [0.0, 0.0],
                };

                material_infos.push(material_info);
            }
        }

        // If no textures exist, use a small fallback so you don’t create an empty buffer
        let fallback_data = vec![0];
        let texture_data = if all_texture_data.is_empty() {
            fallback_data
        } else {
            all_texture_data
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

        let texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: bytemuck::cast_slice(&texture_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 3) Create the texture info buffer
        let material_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&material_infos),
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
            texture_info_buffer: material_info_buffer,
            tile_buffer,
            triangle_list_buffer,
            partial_sums_buffer,
        }
    }
}
