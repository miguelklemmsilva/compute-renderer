// -----------------------------------------------------------------------------
// RASTER STAGE
// -----------------------------------------------------------------------------

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

struct Fragment {
    screen_x: u32,
    screen_y: u32,
    depth: atomic<u32>,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    texture_index: u32,
};

struct FragmentBuffer {
    frags: array<Fragment>,
};

struct UniformRaster {
    width: f32,
    height: f32,
}

struct EffectUniform {
    effect_type: u32,
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    time: f32,
    _padding: vec2<f32>,
}

// -- BINDINGS
@group(0) @binding(0) var<storage, read> projected_buffer: ProjectedVertexBuffer;
@group(0) @binding(1) var<storage, read_write> fragment_buffer: FragmentBuffer;

@group(1) @binding(0) var<uniform> screen_dims: UniformRaster;

@group(2) @binding(0) var<uniform> effect: EffectUniform;

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

fn float_to_depth_int(depth: f32) -> u32 {
    // Ensure we handle the full range properly
    return u32(depth * 4294967295.0);
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

            if effect.effect_type == 2u {
                // We animate meltdown with time. For a triangle to be drawn,
                // min(bc.x, bc.y, bc.z) must exceed some threshold.
                // That threshold can be something that grows or shrinks over time.
                let amplitude = effect.param1;
                let phase = effect.param2;

                let wave = 0.5 + 0.5 * sin(effect.time + phase);   // ranges [0..1]
                let meltdownThreshold = amplitude * wave;

                let min_bc = min(bc.x, min(bc.y, bc.z));
                if min_bc < meltdownThreshold {
                    continue;
                }
            }

            // Perspective correction
            let one_over_w1 = 1.0 / v1.w_clip;
            let one_over_w2 = 1.0 / v2.w_clip;
            let one_over_w3 = 1.0 / v3.w_clip;

            let interpolated_one_over_w = bc.x * one_over_w1 + bc.y * one_over_w2 + bc.z * one_over_w3;

            let z_over_w1 = bc.x * v1.z * one_over_w1;
            let z_over_w2 = bc.y * v2.z * one_over_w2;
            let z_over_w3 = bc.z * v3.z * one_over_w3;
            let interpolated_z = (z_over_w1 + z_over_w2 + z_over_w3) / interpolated_one_over_w;
            let depth = clamp(interpolated_z, 0.0, 1.0);

            // Do an atomic depth test with the per-pixel buffer
            let pixel_id = x + y * u32(screen_dims.width);
            let new_depth_int = float_to_depth_int(depth);

            loop {
                let old_depth = atomicLoad(&fragment_buffer.frags[pixel_id].depth);
                if new_depth_int < old_depth {
                    let exchanged = atomicCompareExchangeWeak(&fragment_buffer.frags[pixel_id].depth, old_depth, new_depth_int);
                    if exchanged.exchanged {
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

                        // Create a new fragment
                        fragment_buffer.frags[pixel_id].screen_x = x;
                        fragment_buffer.frags[pixel_id].screen_y = y;
                        fragment_buffer.frags[pixel_id].uv = uv;
                        fragment_buffer.frags[pixel_id].normal = interpolated_normal;
                        fragment_buffer.frags[pixel_id].world_pos = interpolated_world_pos;
                        fragment_buffer.frags[pixel_id].texture_index = v1.texture_index;

                        atomicExchange(&fragment_buffer.frags[pixel_id].depth, new_depth_int);
                        break;
                    }
                } else {
                    // The fragment is behind the one already in the buffer; ignore it.
                    break;
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
@compute @workgroup_size(256)
fn raster_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_idx = global_id.x * 3u;

    if triangle_idx + 2u >= arrayLength(&projected_buffer.values) {
        return;
    }

    let v1 = projected_buffer.values[triangle_idx];
    let v2 = projected_buffer.values[triangle_idx + 1u];
    let v3 = projected_buffer.values[triangle_idx + 2u];

        let a = vec2<f32>(v2.x - v1.x, v2.y - v1.y);
    let b = vec2<f32>(v3.x - v1.x, v3.y - v1.y);
    let cross_z = a.x * b.y - a.y * b.x;

    // If cross_z <= 0, this triangle is back-facing (depending on your winding convention).
    // We'll skip rasterizing in that case. 
    if cross_z >= 0.0 {
        return;
    }

    rasterize_triangle(v1, v2, v3);
}