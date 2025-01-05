// -----------------------------------------------------------------------------
// RASTER STAGE
// -----------------------------------------------------------------------------
// This stage figures out which screen pixels belong to which triangles, plus
// all the interpolated data needed in the fragment stage. Instead of calling
// color_pixel immediately, we'll store "fragments" to a buffer. The fragment
// pass will then do shading + depth test.

struct Uniform {
    width: f32,
    height: f32,
};

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    u: f32,
    v: f32,
    nx: f32,  // storing world pos x
    ny: f32,  // storing world pos y
    nz: f32,  // storing world pos z
    texture_index: u32,
    w_clip: f32,
};

struct ProjectedVertexBuffer {
    values: array<Vertex>,
};

// Stores each "fragment candidate" that we'll pass to the fragment stage.
struct Fragment {
    screen_x: u32,
    screen_y: u32,
    depth: f32,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    texture_index: u32,
};

// We'll store all fragments in a giant array. In practice, you might need
// a more sophisticated approach (atomic bins, tile-based, etc.) to avoid
// huge memory overhead. This is just to illustrate the separation of stages.
struct FragmentBuffer {
    frags: array<Fragment>,
};

// We need an atomic counter to know how many fragments we've emitted so far.
struct FragmentCounter {
    counter: atomic<u32>,
};

struct UniformRaster {
    // If you want any additional raster-related uniform data, put it here.
    // In this simple example, we just reuse the screen dims from your original code.
    width: f32,
    height: f32,
}

// -- BINDINGS
@group(0) @binding(0) var<storage, read> projected_buffer: ProjectedVertexBuffer;
@group(0) @binding(1) var<storage, read_write> fragment_buffer: FragmentBuffer;
@group(0) @binding(2) var<storage, read_write> fragment_count: FragmentCounter;

@group(1) @binding(0) var<uniform> screen_dims: UniformRaster;

// -----------------------------------------------------------------------------
// HELPERS
// -----------------------------------------------------------------------------

fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    var min_max = vec4<f32>();
    min_max.x = min(min(v1.x, v2.x), v3.x);
    min_max.y = min(min(v1.y, v2.y), v3.y);
    min_max.z = max(max(v1.x, v2.x), v3.x);
    min_max.w = max(max(v1.y, v2.y), v3.y);
    return min_max;
}

fn barycentric(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    let u = cross(
        vec3<f32>(v3.x - v1.x, v2.x - v1.x, v1.x - p.x),
        vec3<f32>(v3.y - v1.y, v2.y - v1.y, v1.y - p.y)
    );
    if abs(u.z) < 1.0 {
        return vec3<f32>(-1.0, 1.0, 1.0);
    }
    return vec3<f32>(1.0 - (u.x + u.y) / u.z, u.y / u.z, u.x / u.z);
}

// Rasterizes a single triangle, pushing fragments out to the fragment buffer.
fn rasterize_triangle(v1: Vertex, v2: Vertex, v3: Vertex) {
    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

    let startX = u32(clamp(min_max.x, 0.0, screen_dims.width - 1.0));
    let startY = u32(clamp(min_max.y, 0.0, screen_dims.height - 1.0));
    let endX   = u32(clamp(min_max.z, 0.0, screen_dims.width - 1.0));
    let endY   = u32(clamp(min_max.w, 0.0, screen_dims.height - 1.0));

    let world_pos1 = vec3<f32>(v1.nx, v1.ny, v1.nz);
    let world_pos2 = vec3<f32>(v2.nx, v2.ny, v2.nz);
    let world_pos3 = vec3<f32>(v3.nx, v3.ny, v3.nz);

    for (var x: u32 = startX; x <= endX; x = x + 1u) {
        for (var y: u32 = startY; y <= endY; y = y + 1u) {
            let bc = barycentric(
                vec3<f32>(v1.x, v1.y, v1.z),
                vec3<f32>(v2.x, v2.y, v2.z),
                vec3<f32>(v3.x, v3.y, v3.z),
                vec2<f32>(f32(x), f32(y))
            );
            if bc.x < 0.0 || bc.y < 0.0 || bc.z < 0.0 {
                continue;
            }

            // Perspective correction
            let one_over_w1 = 1.0 / v1.w_clip;
            let one_over_w2 = 1.0 / v2.w_clip;
            let one_over_w3 = 1.0 / v3.w_clip;

            let interpolated_one_over_w =
                bc.x * one_over_w1 + bc.y * one_over_w2 + bc.z * one_over_w3;

            let uv_over_w1 = bc.x * vec2<f32>(v1.u, v1.v) * one_over_w1;
            let uv_over_w2 = bc.y * vec2<f32>(v2.u, v2.v) * one_over_w2;
            let uv_over_w3 = bc.z * vec2<f32>(v3.u, v3.v) * one_over_w3;
            let interpolated_uv_over_w = uv_over_w1 + uv_over_w2 + uv_over_w3;
            let uv = interpolated_uv_over_w / interpolated_one_over_w;

            let norm_over_w1 = bc.x * vec3<f32>(v1.nx, v1.ny, v1.nz) * one_over_w1;
            let norm_over_w2 = bc.y * vec3<f32>(v2.nx, v2.ny, v2.nz) * one_over_w2;
            let norm_over_w3 = bc.z * vec3<f32>(v3.nx, v3.ny, v3.nz) * one_over_w3;
            let interpolated_normal = normalize((norm_over_w1 + norm_over_w2 + norm_over_w3)
                                                 / interpolated_one_over_w);

            let pos_over_w1 = bc.x * world_pos1 * one_over_w1;
            let pos_over_w2 = bc.y * world_pos2 * one_over_w2;
            let pos_over_w3 = bc.z * world_pos3 * one_over_w3;
            let interpolated_world_pos = (pos_over_w1 + pos_over_w2 + pos_over_w3)
                                         / interpolated_one_over_w;

            let z_over_w1 = bc.x * v1.z * one_over_w1;
            let z_over_w2 = bc.y * v2.z * one_over_w2;
            let z_over_w3 = bc.z * v3.z * one_over_w3;
            let interpolated_z = (z_over_w1 + z_over_w2 + z_over_w3) / interpolated_one_over_w;
            let depth = clamp(interpolated_z, 0.0, 1.0);

            // Create a new fragment
            let frag_index = atomicAdd(&fragment_count.counter, 1u);
            fragment_buffer.frags[frag_index] = Fragment(
                x,
                y,
                depth,
                uv,
                interpolated_normal,
                interpolated_world_pos,
                v1.texture_index
            );
        }
    }
}

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
@compute @workgroup_size(256)
fn raster_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_idx = global_id.x * 3u;

    // We assume the same # of vertices in projected_buffer as in the original vertex buffer.
    if triangle_idx + 2u >= arrayLength(&projected_buffer.values) {
        return;
    }

    let v1 = projected_buffer.values[triangle_idx];
    let v2 = projected_buffer.values[triangle_idx + 1u];
    let v3 = projected_buffer.values[triangle_idx + 2u];

    rasterize_triangle(v1, v2, v3);
}