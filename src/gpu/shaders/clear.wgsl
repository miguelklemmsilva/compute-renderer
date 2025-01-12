// -----------------------------------------------------------------------------
// CLEAR STAGE
// -----------------------------------------------------------------------------
// Efficiently clears both the output and depth buffers in parallel.

struct OutputBuffer {
    data: array<atomic<u32>>,
};

struct TileTriangles {
    count: atomic<u32>,
    offset: u32, 
    write_index: atomic<u32>,
    padding: u32,
};

struct TileBuffer {
    triangle_indices: array<TileTriangles>,
}

struct TriangleListBuffer {
    indices: array<u32>,
}

struct Fragment {
    depth: atomic<u32>,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    texture_index: u32,
};

struct FragmentBuffer {
    frags: array<Fragment>,
};

struct Uniform {
    width: f32,
    height: f32,
};

struct PartialSums {
    values: array<u32>,
}

@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;
@group(0) @binding(1) var<storage, read_write> fragment_buffer: FragmentBuffer;
@group(0) @binding(2) var<storage, read_write> tile_buffer: TileBuffer;
@group(0) @binding(3) var<storage, read_write> triangle_list_buffer: TriangleListBuffer;
@group(0) @binding(4) var<storage, read_write> partial_sums: PartialSums;

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
        atomicStore(&output_buffer.data[idx], 0u);

        // Clear fragment buffer
        atomicStore(&fragment_buffer.frags[idx].depth, 0xFFFFFFFFu);
        fragment_buffer.frags[idx].uv = vec2<f32>(0.0, 0.0);
        fragment_buffer.frags[idx].normal = vec3<f32>(0.0, 0.0, 0.0);
        fragment_buffer.frags[idx].world_pos = vec3<f32>(0.0, 0.0, 0.0);
        fragment_buffer.frags[idx].texture_index = 0u;
    }

    // Clear tile buffer and set up offsets - ensure we clear all tiles
    if idx < total_tiles {
        // Ensure complete reset of tile data
        atomicStore(&tile_buffer.triangle_indices[idx].count, 0u);
        tile_buffer.triangle_indices[idx].offset = 0u;
        atomicStore(&tile_buffer.triangle_indices[idx].write_index, 0u);
        tile_buffer.triangle_indices[idx].padding = 0u;  // Clear padding just to be safe
    }

    // Clear partial sums buffer - ensure we clear enough entries for all workgroups
    if idx < num_workgroups {
        partial_sums.values[idx] = 0u;
    }
} 
