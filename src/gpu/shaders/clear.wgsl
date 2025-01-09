// -----------------------------------------------------------------------------
// CLEAR STAGE
// -----------------------------------------------------------------------------
// Efficiently clears both the output and depth buffers in parallel.

const MAX_TRIANGLES_PER_TILE: u32 = 1024u;

struct OutputBuffer {
    data: array<atomic<u32>>,
};

struct DepthBuffer {
    depth: array<atomic<u32>>,
};

struct TileTriangles {
    count: atomic<u32>, 
    triangle_indices: array<u32, MAX_TRIANGLES_PER_TILE>,
};

struct TileBuffer {
    triangle_indices: array<TileTriangles>,
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

@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;
@group(0) @binding(1) var<storage, read_write> fragment_buffer: FragmentBuffer;
@group(0) @binding(2) var<storage, read_write> tile_buffer: TileBuffer;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
// Uses a large workgroup size to maximize parallelism and throughput
@compute @workgroup_size(256)
fn clear_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_pixels = u32(screen_dims.width) * u32(screen_dims.height);
    
    // Early exit if we're beyond the screen dimensions
    if idx >= total_pixels {
        return;
    }

    // Clear color buffer to black (0x000000)
    atomicStore(&output_buffer.data[idx], 0u);

    atomicStore(&fragment_buffer.frags[idx].depth, 0xFFFFFFFFu);
    fragment_buffer.frags[idx].uv = vec2<f32>(0.0, 0.0);
    fragment_buffer.frags[idx].normal = vec3<f32>(0.0, 0.0, 0.0);
    fragment_buffer.frags[idx].world_pos = vec3<f32>(0.0, 0.0, 0.0);
    fragment_buffer.frags[idx].texture_index = 0u;
    
    atomicStore(&tile_buffer.triangle_indices[idx].count, 0u);

    tile_buffer.triangle_indices[idx].triangle_indices = array<u32, MAX_TRIANGLES_PER_TILE>();
} 
