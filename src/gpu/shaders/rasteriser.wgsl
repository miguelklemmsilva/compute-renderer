// -----------------------------------------------------------------------------
// RASTER STAGE
// -----------------------------------------------------------------------------

const TILE_SIZE: u32 = 16u;  // Size of a tile in pixels

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
    return u32(depth * 4294967295.0);
}

// Check if a triangle overlaps with a tile
fn triangle_overlaps_tile(min_max: vec4<f32>, tile_x: u32, tile_y: u32) -> bool {
    let tile_min_x = f32(tile_x * TILE_SIZE);
    let tile_min_y = f32(tile_y * TILE_SIZE);
    let tile_max_x = f32((tile_x + 1u) * TILE_SIZE);
    let tile_max_y = f32((tile_y + 1u) * TILE_SIZE);
    
    // Check if the triangle's bounding box overlaps with the tile's bounding box
    return !(min_max.z < tile_min_x || min_max.x > tile_max_x || min_max.w < tile_min_y || min_max.y > tile_max_y);
}

// Rasterizes a triangle within a specific tile
fn rasterize_triangle_in_tile(v1: Vertex, v2: Vertex, v3: Vertex, tile_x: u32, tile_y: u32) {
    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

    // Early exit if triangle doesn't overlap this tile
    if !triangle_overlaps_tile(min_max, tile_x, tile_y) {
        return;
    }

    // Calculate tile bounds
    let tile_start_x = tile_x * TILE_SIZE;
    let tile_start_y = tile_y * TILE_SIZE;
    let tile_end_x = min(tile_start_x + TILE_SIZE, u32(screen_dims.width));
    let tile_end_y = min(tile_start_y + TILE_SIZE, u32(screen_dims.height));

    // Clamp to tile boundaries and screen dimensions
    let startX = u32(max(f32(tile_start_x), min_max.x));
    let startY = u32(max(f32(tile_start_y), min_max.y));
    let endX = u32(min(f32(tile_end_x), min_max.z + 1.0));  // Add 1 to include the last pixel
    let endY = u32(min(f32(tile_end_y), min_max.w + 1.0));  // Add 1 to include the last pixel

    let world_pos1 = vec3<f32>(v1.nx, v1.ny, v1.nz);
    let world_pos2 = vec3<f32>(v2.nx, v2.ny, v2.nz);
    let world_pos3 = vec3<f32>(v3.nx, v3.ny, v3.nz);

    let one_over_w1 = 1.0 / v1.w_clip;
    let one_over_w2 = 1.0 / v2.w_clip;
    let one_over_w3 = 1.0 / v3.w_clip;

    for (var x: u32 = startX; x <= endX; x = x + 1u) {
        for (var y: u32 = startY; y <= endY; y = y + 1u) {
            var bc = barycentric(
                vec3<f32>(v1.x, v1.y, v1.z),
                vec3<f32>(v2.x, v2.y, v2.z),
                vec3<f32>(v3.x, v3.y, v3.z),
                vec2<f32>(f32(x), f32(y))
            );

            var threshold = 0.0;

            if effect.effect_type == 3u { // voxelization
                threshold = -effect.param1;
            }

            if bc.x < threshold || bc.y < threshold || bc.z < threshold {
                continue;
            }


            if effect.effect_type == 2u { // melt effect
                let amplitude = effect.param1;
                let phase = effect.param2;
                let wave = 0.5 + 0.5 * sin(effect.time + phase);
                let meltdownThreshold = amplitude * wave;
                let min_bc = min(bc.x, min(bc.y, bc.z));
                if min_bc < meltdownThreshold {
                    continue;
                }
            }

            // Only calculate perspective correction if we actually need to draw this pixel
            let interpolated_one_over_w = bc.x * one_over_w1 + bc.y * one_over_w2 + bc.z * one_over_w3;

            let z_over_w1 = bc.x * v1.z * one_over_w1;
            let z_over_w2 = bc.y * v2.z * one_over_w2;
            let z_over_w3 = bc.z * v3.z * one_over_w3;
            let interpolated_z = (z_over_w1 + z_over_w2 + z_over_w3) / interpolated_one_over_w;
            let depth = clamp(interpolated_z, 0.0, 1.0);

            let pixel_id = x + y * u32(screen_dims.width);
            let new_depth_int = float_to_depth_int(depth);

            // Early depth test before doing expensive interpolations
            let old_depth = atomicLoad(&fragment_buffer.frags[pixel_id].depth);
            if new_depth_int >= old_depth {
                continue;
            }

            atomicStore(&fragment_buffer.frags[pixel_id].depth, new_depth_int);
            let uv_over_w1 = bc.x * vec2<f32>(v1.u, v1.v) * one_over_w1;
            let uv_over_w2 = bc.y * vec2<f32>(v2.u, v2.v) * one_over_w2;
            let uv_over_w3 = bc.z * vec2<f32>(v3.u, v3.v) * one_over_w3;
            let interpolated_uv_over_w = uv_over_w1 + uv_over_w2 + uv_over_w3;
            let uv = interpolated_uv_over_w / interpolated_one_over_w;

            let norm_over_w1 = bc.x * vec3<f32>(v1.nx, v1.ny, v1.nz) * one_over_w1;
            let norm_over_w2 = bc.y * vec3<f32>(v2.nx, v2.ny, v2.nz) * one_over_w2;
            let norm_over_w3 = bc.z * vec3<f32>(v3.nx, v3.ny, v3.nz) * one_over_w3;
            let interpolated_normal = normalize((norm_over_w1 + norm_over_w2 + norm_over_w3) / interpolated_one_over_w);

            let pos_over_w1 = bc.x * world_pos1 * one_over_w1;
            let pos_over_w2 = bc.y * world_pos2 * one_over_w2;
            let pos_over_w3 = bc.z * world_pos3 * one_over_w3;
            let interpolated_world_pos = (pos_over_w1 + pos_over_w2 + pos_over_w3) / interpolated_one_over_w;

            fragment_buffer.frags[pixel_id].uv = uv;
            fragment_buffer.frags[pixel_id].normal = interpolated_normal;
            fragment_buffer.frags[pixel_id].world_pos = interpolated_world_pos;
            fragment_buffer.frags[pixel_id].texture_index = v1.texture_index;
        }
    }
}

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
@compute @workgroup_size(16, 16)
fn raster_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Calculate which tile this workgroup is processing
    let tile_x = global_id.x;
    let tile_y = global_id.y;
    
    // Early exit if this tile is outside the screen
    if tile_x >= (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE || tile_y >= (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE {
        return;
    }

    // Calculate tile bounds for quick triangle rejection
    let tile_min_x = f32(tile_x * TILE_SIZE);
    let tile_min_y = f32(tile_y * TILE_SIZE);
    let tile_max_x = f32((tile_x + 1u) * TILE_SIZE);
    let tile_max_y = f32((tile_y + 1u) * TILE_SIZE);

    // Process each triangle
    for (var i = 0u; i < arrayLength(&projected_buffer.values); i = i + 3u) {
        let v1 = projected_buffer.values[i];
        let v2 = projected_buffer.values[i + 1u];
        let v3 = projected_buffer.values[i + 2u];

        // Quick AABB test before doing more expensive tests
        let min_x = min(min(v1.x, v2.x), v3.x);
        let max_x = max(max(v1.x, v2.x), v3.x);
        let min_y = min(min(v1.y, v2.y), v3.y);
        let max_y = max(max(v1.y, v2.y), v3.y);

        // Skip triangle if it's completely outside this tile
        if max_x < tile_min_x || min_x > tile_max_x || max_y < tile_min_y || min_y > tile_max_y {
            continue;
        }

        let a = vec2<f32>(v2.x - v1.x, v2.y - v1.y);
        let b = vec2<f32>(v3.x - v1.x, v3.y - v1.y);
        let cross_z = a.x * b.y - a.y * b.x;

        // Skip back-facing triangles
        if cross_z >= 0.0 {
            continue;
        }

        rasterize_triangle_in_tile(v1, v2, v3, tile_x, tile_y);
    }
}