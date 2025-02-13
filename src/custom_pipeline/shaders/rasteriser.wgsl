// ---------------------------------------------------------------------
// This shader implements adaptive tile splitting for rasterization.
// When a tile has many triangles (for example, when the camera is far away),
// each tile’s triangle list is processed in parallel by a workgroup of 64 threads.
// ---------------------------------------------------------------------

const TILE_SIZE: u32 = 8u;

struct UniformRaster {
    width: f32,
    height: f32,
};

struct EffectUniform {
    effect_type: u32,
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    time: f32,
    _padding: vec2<f32>,
};

struct Vertex {
    world_pos: vec4<f32>,
    normal: vec4<f32>,
    uv: vec2<f32>,
};

struct Fragment {
    depth: atomic<u32>,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    texture_index: u32,
};

struct TileTriangles {
    count: atomic<u32>,
    offset: u32,
    write_index: atomic<u32>,
    padding: u32
}

@group(0) @binding(0)
var<storage, read> projected_buffer: array<Vertex>;

@group(0) @binding(1)
var<storage, read_write> fragment_buffer: array<Fragment>;

@group(0) @binding(2)
var<storage, read> tile_buffer: array<TileTriangles>;

@group(0) @binding(3)
var<storage, read> triangle_list_buffer: array<u32>;

@group(0) @binding(4)
var<storage, read> indices: array<u32>;

@group(1) @binding(0)
var<uniform> screen_dims: UniformRaster;

@group(2) @binding(0)
var<uniform> effect: EffectUniform;

// ---------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------

// Compute barycentric coordinates for point p (in 2D screen space)
fn barycentric(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    let u = cross(
        vec3<f32>(v3.x - v1.x, v2.x - v1.x, v1.x - p.x),
        vec3<f32>(v3.y - v1.y, v2.y - v1.y, v1.y - p.y)
    );
    if abs(u.z) < 1e-5 {
        return vec3<f32>(-1.0, 1.0, 1.0);
    }
    return vec3<f32>(1.0 - (u.x + u.y) / u.z, u.y / u.z, u.x / u.z);
}

// Simple min/max helper for 3 points (used to compute the screen-space bounding box)
fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    let min_x = min(min(v1.x, v2.x), v3.x);
    let min_y = min(min(v1.y, v2.y), v3.y);
    let max_x = max(max(v1.x, v2.x), v3.x);
    let max_y = max(max(v1.y, v2.y), v3.y);
    return vec4<f32>(min_x, min_y, max_x, max_y);
}

// Pack and unpack functions for depth values.
fn pack_float_to_u32(value: f32) -> u32 {
    return bitcast<u32>(value);
}

fn unpack_u32_to_float(bits: u32) -> f32 {
    return bitcast<f32>(bits);
}

// ---------------------------------------------------------------------
// Rasterization function: rasterize a triangle into one tile.
// The triangle’s vertices are in screen space and already have their
// perspective divide (and attributes pre–divided by w) applied.
// ---------------------------------------------------------------------
fn rasterize_triangle_in_tile(v1: Vertex, v2: Vertex, v3: Vertex, tile_x: u32, tile_y: u32) {
    // Compute the pixel bounds for the tile.
    let tile_start_x = tile_x * TILE_SIZE;
    let tile_end_x = min(tile_start_x + TILE_SIZE, u32(screen_dims.width));
    let tile_start_y = tile_y * TILE_SIZE;
    let tile_end_y = min(tile_start_y + TILE_SIZE, u32(screen_dims.height));

    // Pre–compute reciprocal w for perspective–correct interpolation.
    let one_over_w1 = 1.0 / v1.world_pos.w;
    let one_over_w2 = 1.0 / v2.world_pos.w;
    let one_over_w3 = 1.0 / v3.world_pos.w;

    // Pre–divide attributes.
    let world_pos1 = v1.normal * one_over_w1;
    let world_pos2 = v2.normal * one_over_w2;
    let world_pos3 = v3.normal * one_over_w3;

    let normal1 = v1.normal * one_over_w1;
    let normal2 = v2.normal * one_over_w2;
    let normal3 = v3.normal * one_over_w3;

    let uv1 = v1.uv * one_over_w1;
    let uv2 = v2.uv * one_over_w2;
    let uv3 = v3.uv * one_over_w3;

    // Pre-divide depth values.
    let z1 = v1.world_pos.z * one_over_w1;
    let z2 = v2.world_pos.z * one_over_w2;
    let z3 = v3.world_pos.z * one_over_w3;

    // Loop over the pixels in the tile.
    for (var x: u32 = tile_start_x; x < tile_end_x; x = x + 1u) {
        for (var y: u32 = tile_start_y; y < tile_end_y; y = y + 1u) {
            let bc = barycentric(
                v1.world_pos.xyz,
                v2.world_pos.xyz,
                v3.world_pos.xyz,
                vec2<f32>(f32(x), f32(y))
            );

            var threshold = 0.0;
            if effect.effect_type == 3u {
                threshold = -effect.param1;
            }

            if bc.x < threshold || bc.y < threshold || bc.z < threshold {
                continue;
            }

            if effect.effect_type == 2u {
                let amplitude = effect.param1;
                let phase = effect.param2;
                let wave = 0.5 + 0.5 * sin(effect.time + phase);
                let meltdownThreshold = amplitude * wave;
                let min_bc = min(bc.x, min(bc.y, bc.z));
                if min_bc < meltdownThreshold {
                    continue;
                }
            }
            
            // Interpolate depth.
            let interpolated_z = bc.x * z1 + bc.y * z2 + bc.z * z3;
            if interpolated_z < -1.0 || interpolated_z > 1.0 {
                continue;
            }
            
            // Convert to [0,1] range for depth buffer
            let depth = interpolated_z;
            let pixel_id = x + y * u32(screen_dims.width);
            // Convert our computed depth to a packed u32.
            let packed_depth = pack_float_to_u32(depth);
            // Get the pointer for the current pixel.
            let pixel_ptr = &fragment_buffer[pixel_id].depth;

            // Attempt an atomic update in a loop.
            var old = atomicLoad(pixel_ptr);
            loop {
                let old_depth = unpack_u32_to_float(old);
                if depth >= old_depth {
                    // Our new depth is not better than the one already stored.
                    break;
                }
                // Try to atomically update the depth.
                let result = atomicCompareExchangeWeak(pixel_ptr, old, packed_depth);
                if result.exchanged {
                    // Interpolate UV coordinates (already divided by w)
                    let interpolated_uv = bc.x * uv1 + bc.y * uv2 + bc.z * uv3;
                    let interpolated_world_pos = bc.x * world_pos1 + bc.y * world_pos2 + bc.z * world_pos3;
                    let interpolated_normal = normalize(bc.x * normal1 + bc.y * normal2 + bc.z * normal3);

                    // We won the race: update the fragment data.
                    fragment_buffer[pixel_id].uv = interpolated_uv;
                    fragment_buffer[pixel_id].normal = interpolated_normal.xyz;
                    fragment_buffer[pixel_id].world_pos = interpolated_world_pos.xyz;
                    break;
                }
                
                // Otherwise, update old and try again.
                old = result.old_value;
            }
        }
    }
}

@compute @workgroup_size(1, 1, 256)
fn raster_main(@builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>) {
    // Use the workgroup ID to determine the tile.
    let tile_x = wg.x;
    let tile_y = wg.y;

    // Compute total number of tiles along x and y.
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;
    
    // Early exit if this tile is out-of-range.
    if tile_x >= num_tiles_x || tile_y >= num_tiles_y {
        return;
    }

    let tile_idx = tile_x + tile_y * num_tiles_x;
    let triangle_count = atomicLoad(&tile_buffer[tile_idx].count);
    let triangle_offset = tile_buffer[tile_idx].offset;
    
    // Use the third dimension of the local invocation to split work.
    let thread_index = lid.z;

    for (var i = thread_index; i < triangle_count; i += 256u) {
        let base_idx = triangle_list_buffer[triangle_offset + i];
        let idx1 = indices[base_idx];
        let idx2 = indices[base_idx + 1u];
        let idx3 = indices[base_idx + 2u];
        let v1 = projected_buffer[idx1];
        let v2 = projected_buffer[idx2];
        let v3 = projected_buffer[idx3];

        // Discard triangles with any vertex behind the near plane.
        if v1.world_pos.w < 0.0 || v2.world_pos.w < 0.0 || v3.world_pos.w < 0.0 {
            continue;
        }
        
        // Back-face culling (unless the effect requires both sides).
        let a = vec2<f32>(v2.world_pos.x - v1.world_pos.x, v2.world_pos.y - v1.world_pos.y);
        let b = vec2<f32>(v3.world_pos.x - v1.world_pos.x, v3.world_pos.y - v1.world_pos.y);
        let cross_z = a.x * b.y - a.y * b.x;
        if effect.effect_type != 3u && cross_z >= 0.0 {
            continue;
        }

        rasterize_triangle_in_tile(v1, v2, v3, tile_x, tile_y);
    }
}