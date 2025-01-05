// -----------------------------------------------------------------------------
// CLEAR STAGE
// -----------------------------------------------------------------------------
// Efficiently clears both the output and depth buffers in parallel.

struct OutputBuffer {
    data: array<atomic<u32>>,
};

struct DepthBuffer {
    depth: array<atomic<u32>>,
};

struct FragmentCounter {
    counter: atomic<u32>,
};

struct Fragment {
    screen_x: u32,
    screen_y: u32,
    depth: f32,
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
@group(0) @binding(1) var<storage, read_write> depth_buffer: DepthBuffer;
@group(0) @binding(2) var<storage, read_write> fragment_counter: FragmentCounter;
@group(0) @binding(3) var<storage, read_write> fragment_buffer: FragmentBuffer;

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
    
    // Clear depth buffer to maximum depth (0xFFFFFFFF)
    atomicStore(&depth_buffer.depth[idx], 0xFFFFFFFFu);

    // Clear fragment buffer entry (set to default/empty values)
    fragment_buffer.frags[idx] = Fragment(
        0u,        // screen_x
        0u,        // screen_y
        0.0,       // depth
        vec2<f32>(0.0, 0.0),  // uv
        vec3<f32>(0.0, 0.0, 0.0),  // normal
        vec3<f32>(0.0, 0.0, 0.0),  // world_pos
        0xFFFFFFFFu  // texture_index (invalid)
    );

    // Only thread 0 needs to clear the fragment counter
    if idx == 0u {
        atomicStore(&fragment_counter.counter, 0u);
    }
} 