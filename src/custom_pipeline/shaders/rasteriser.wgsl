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
    uv: vec2<f32>,
    normal: vec4<f32>,
    world_pos: vec4<f32>,
};

struct TileTriangles {
    count: atomic<u32>,
    offset: u32,
    write_index: atomic<u32>,
    padding: u32
}

struct TriangleBinningData {
    min_max: vec4<f32>,
    start_tile: vec2<u32>,
    tile_range: vec2<u32>,
};

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

@group(0) @binding(5)
var<storage, read> triangle_binning_buffer: array<TriangleBinningData>;

@group(0) @binding(6)
var <storage, read_write> depth_buffer: array<atomic<u32>>;

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
    return vec3<f32>(
        1.0 - (u.x + u.y) / u.z,
        u.y / u.z,
        u.x / u.z
    );
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

    // Loop over the pixels in the tile.
    for (var x: u32 = tile_start_x; x < tile_end_x; x++) {
        for (var y: u32 = tile_start_y; y < tile_end_y; y++) {
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
            let interpolated_z = bc.x * v1.world_pos.z + bc.y * v2.world_pos.z + bc.z * v3.world_pos.z;
            
            // Convert to [0,1] range for depth buffer
            let depth = interpolated_z * 0.5 + 0.5;
            let pixel_id = x + y * u32(screen_dims.width);
            // Convert our computed depth to a packed u32.
            let packed_depth = pack_float_to_u32(depth);
            // Get the pointer for the current pixel.
            let pixel_ptr = &depth_buffer[pixel_id];

            // Attempt an atomic update in a loop.
            var old = atomicLoad(pixel_ptr);
            loop {
                if packed_depth >= old {
                    // Our computed depth is not closer.
                    break;
                }

                // sporadic atomic change failures occur if there is no barrier
                storageBarrier();

                let result = atomicCompareExchangeWeak(pixel_ptr, old, packed_depth);

                // Try to atomically update the depth.
                if result.exchanged {
                    // Interpolate UV coordinates (already divided by w)
                    let interpolated_uv = bc.x * v1.uv + bc.y * v2.uv + bc.z * v3.uv;
                    let interpolated_world_pos = bc.x * v1.world_pos + bc.y * v2.world_pos + bc.z * v3.world_pos;
                    let interpolated_normal = bc.x * v1.normal + bc.y * v2.normal + bc.z * v3.normal;

                    // We won the race: update the fragment data.
                    fragment_buffer[pixel_id].uv = interpolated_uv;
                    fragment_buffer[pixel_id].normal = interpolated_normal;
                    fragment_buffer[pixel_id].world_pos = interpolated_world_pos;
                    break;
                }
                
                // Otherwise, update old and try again.
                old = result.old_value;
            }
        }
    }
}

@compute @workgroup_size(1, 1, 64)
fn raster_main(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>
) {
    // Determine the tile for this workgroup.
    let tile_x = wg.x;
    let tile_y = wg.y;
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;

    // Early exit if this tile is out of range.
    if tile_x >= num_tiles_x || tile_y >= num_tiles_y {
        return;
    }

    let tile_idx = tile_x + tile_y * num_tiles_x;
    let triangle_count = atomicLoad(&tile_buffer[tile_idx].count);
    let triangle_offset = tile_buffer[tile_idx].offset;
    
    // Use the third dimension of the local invocation to split work.
    let thread_index = lid.z;
    for (var i = thread_index; i < triangle_count; i += 64u) {
        // Get the triangle's base index from the triangle list.
        let base_idx = triangle_list_buffer[triangle_offset + i];
        // Compute the triangle index from the base index.
        let triangle_index = base_idx / 3u;
        // Load precomputed metadata.
        let triangle_meta = triangle_binning_buffer[triangle_index];

        // Retrieve the vertex indices.
        let idx1 = indices[base_idx];
        let idx2 = indices[base_idx + 1u];
        let idx3 = indices[base_idx + 2u];
        let v1 = projected_buffer[idx1];
        let v2 = projected_buffer[idx2];
        let v3 = projected_buffer[idx3];
        
        // Back-face culling (unless the effect requires both sides).
        let a = vec2<f32>(v2.world_pos.x - v1.world_pos.x, v2.world_pos.y - v1.world_pos.y);
        let b = vec2<f32>(v3.world_pos.x - v1.world_pos.x, v3.world_pos.y - v1.world_pos.y);
        let cross_z = a.x * b.y - a.y * b.x;
        if effect.effect_type != 3u && cross_z >= 0.0 {
            continue;
        }

        // Now rasterize the triangle into this tile.
        rasterize_triangle_in_tile(v1, v2, v3, tile_x, tile_y);
    }
}