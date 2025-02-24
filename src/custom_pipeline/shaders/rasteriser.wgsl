const TILE_SIZE: u32 = 8u;

struct UniformRaster {
    width: f32,
    height: f32,
    num_tiles_x: u32,
    num_tiles_y: u32,
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
    world_pos: vec3<f32>,
    screen_pos: vec4<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
};

struct Fragment {
    uv: vec2<f32>,
    normal: vec3<f32>,
    position: vec3<f32>,
    flag: u32,
};

struct TileTriangles {
    count: u32,
    offset: u32,
    write_index: u32,
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
var<storage, read_write> tile_buffer: array<TileTriangles>;

@group(0) @binding(3)
var<storage, read> triangle_list_buffer: array<u32>;

@group(0) @binding(4)
var<storage, read> indices: array<u32>;

@group(0) @binding(5)
var<storage, read> tile_binning_data: array<TriangleBinningData>;

@group(1) @binding(0)
var<uniform> screen_dims: UniformRaster;

@group(2) @binding(0)
var<uniform> effect: EffectUniform;

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

const MAX_TILES: u32 = TILE_SIZE * TILE_SIZE;
var<workgroup> local_depth: array<atomic<u32>, MAX_TILES>;

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
    for (var y = tile_start_y; y < tile_end_y; y++) {
        for (var x = tile_start_x; x < tile_end_x; x++) {
            let bc = barycentric(
                v1.screen_pos.xyz,
                v2.screen_pos.xyz,
                v3.screen_pos.xyz,
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
            let interpolated_z = bc.x * v1.screen_pos.z + bc.y * v2.screen_pos.z + bc.z * v3.screen_pos.z;

            let local_index = (x - tile_start_x) + (y - tile_start_y) * TILE_SIZE;
            let pixel_index = x + y * u32(screen_dims.width);
            // Convert our computed depth to a packed u32.
            let packed_depth = pack_float_to_u32(interpolated_z);
            // Get the pointer for the current pixel.
            let pixel_ptr = &local_depth[local_index];

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
                    // We won the race: update the fragment data.
                    fragment_buffer[pixel_index] = Fragment(
                        bc.x * v1.uv + bc.y * v2.uv + bc.z * v3.uv,
                        bc.x * v1.normal + bc.y * v2.normal + bc.z * v3.normal,
                        bc.x * v1.world_pos + bc.y * v2.world_pos + bc.z * v3.world_pos,
                        1u
                    );
                    break;
                }
                
                // Otherwise, update old and try again.
                old = result.old_value;
            }
        }
    }
}

const Z_DISPATCHES = 64u;

@compute @workgroup_size(1, 1, Z_DISPATCHES)
fn raster_main(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>
) {
    // Determine the tile for this workgroup.
    let tile_x = wg.x;
    let tile_y = wg.y;
    let num_tiles_x = screen_dims.num_tiles_x;
    let num_tiles_y = screen_dims.num_tiles_y;

    local_depth[lid.z] = 0xFFFFFFFFu;
    workgroupBarrier(); // ensure all threads have loaded the data

    // Early exit if this tile is out of range.
    if tile_x >= num_tiles_x || tile_y >= num_tiles_y {
        return;
    }

    let tile_idx = tile_x + tile_y * num_tiles_x;
    let triangle_count = tile_buffer[tile_idx].count;
    let triangle_offset = tile_buffer[tile_idx].offset;
    
    // Use the third dimension of the local invocation to split work.
    for (var i = lid.z; i < triangle_count; i += Z_DISPATCHES) {
        // Get the triangle's base index from the triangle list.
        let base_idx = triangle_list_buffer[triangle_offset + i];

        // Compute the triangle index from the base index.
        let triangle_index = base_idx / 3u;

        let triangle_meta = tile_binning_data[triangle_index];
        if (triangle_meta.tile_range.x * triangle_meta.tile_range.y) == 0u {
            continue;
        }

        // Retrieve the vertex indices.
        let idx1 = indices[base_idx];
        let idx2 = indices[base_idx + 1u];
        let idx3 = indices[base_idx + 2u];
        let v1 = projected_buffer[idx1];
        let v2 = projected_buffer[idx2];
        let v3 = projected_buffer[idx3];

        // Now rasterize the triangle into this tile.
        rasterize_triangle_in_tile(v1, v2, v3, tile_x, tile_y);
    }

    storageBarrier();
    tile_buffer[tile_idx].count = 0u;
    tile_buffer[tile_idx].offset = 0u;
    tile_buffer[tile_idx].write_index = 0u;
}