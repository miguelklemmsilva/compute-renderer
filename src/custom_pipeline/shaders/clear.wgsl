struct TileTriangles {
    count: u32,
    offset: u32,
    write_index: u32,
    padding: u32,
};

struct Fragment {
    depth: u32,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    texture_index: u32,
};

struct Uniform {
    width: f32,
    height: f32,
};

@group(0) @binding(0) var<storage, read_write> output_buffer: array<u32>;
@group(0) @binding(1) var<storage, read_write> fragment_buffer: array<Fragment>;
@group(0) @binding(2) var<storage, read_write> tile_buffer: array<TileTriangles>;
@group(0) @binding(3) var<storage, read_write> triangle_list_buffer: array<u32>;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
// Uses a large workgroup size to maximize parallelism and throughput
@compute @workgroup_size(256)
fn clear_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_pixels = u32(screen_dims.width) * u32(screen_dims.height);
    let num_tiles_x = (u32(screen_dims.width) + 8u - 1u) / 8u;  // TILE_SIZE = 8
    let num_tiles_y = (u32(screen_dims.height) + 8u - 1u) / 8u;
    let total_tiles = num_tiles_x * num_tiles_y;
    let num_workgroups = (total_tiles + 255u) / 256u * 256u;  // Round up to next multiple of workgroup size
    
    // Clear pixel-dependent buffers
    if idx < total_pixels {
        // Clear color buffer to black (0x000000)
        output_buffer[idx] = 0u;

        fragment_buffer[idx].depth = 0xFFFFFFFFu;
    }

    // Clear tile buffer and set up offsets - ensure we clear all tiles
    if idx < total_tiles {
        // Ensure complete reset of tile data
        tile_buffer[idx].count = 0u;
        tile_buffer[idx].offset = 0u;
        tile_buffer[idx].write_index = 0u;
    }
} 
