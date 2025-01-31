const TILE_SIZE: u32 = 8u;

struct Uniform {
    width: f32,
    height: f32,
};

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    u: f32,
    v: f32,
    texture_index: u32,
    w_clip: f32,
};

struct ProjectedVertexBuffer {
    values: array<Vertex>,
};

struct IndexBuffer {
    values: array<u32>,
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

struct TileTriangles {
    count: atomic<u32>,
    offset: u32,  // Offset into triangle list buffer
    write_index: atomic<u32>,
    padding: u32,
};

struct TileBuffer {
    triangle_indices: array<TileTriangles>,
}

struct TriangleListBuffer {
    indices: array<u32>,
}

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
@group(0) @binding(2) var<storage, read> tile_buffer: TileBuffer;
@group(0) @binding(3) var<storage, read> triangle_list_buffer: TriangleListBuffer;
@group(0) @binding(4) var<storage, read> index_buffer: IndexBuffer;

@group(1) @binding(0) var<uniform> screen_dims: UniformRaster;

@group(2) @binding(0) var<uniform> effect: EffectUniform;

// -----------------------------------------------------------------------------
// HELPERS
// -----------------------------------------------------------------------------

fn pack_float_to_u32(value: f32) -> u32 {
    let bits = bitcast<u32>(value);
    return bits;
}

fn unpack_u32_to_float(bits: u32) -> f32 {
    return bitcast<f32>(bits);
}

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

// Rasterizes a triangle within a specific tile
fn rasterize_triangle_in_tile(v1: Vertex, v2: Vertex, v3: Vertex, tile_x: u32, tile_y: u32) {
    // Calculate tile bounds
    let tile_start_x = tile_x * TILE_SIZE;
    let tile_start_y = tile_y * TILE_SIZE;
    let tile_end_x = min(tile_start_x + TILE_SIZE, u32(screen_dims.width));
    let tile_end_y = min(tile_start_y + TILE_SIZE, u32(screen_dims.height));

    // Pre-compute 1/w for perspective correction
    let w1 = v1.w_clip;
    let w2 = v2.w_clip;
    let w3 = v3.w_clip;
    let one_over_w1 = 1.0 / w1;
    let one_over_w2 = 1.0 / w2;
    let one_over_w3 = 1.0 / w3;

    // Pre-divide attributes by w
    let world_pos1 = vec3<f32>(v1.nx, v1.ny, v1.nz) * one_over_w1;
    let world_pos2 = vec3<f32>(v2.nx, v2.ny, v2.nz) * one_over_w2;
    let world_pos3 = vec3<f32>(v3.nx, v3.ny, v3.nz) * one_over_w3;

    let normal1 = vec3<f32>(v1.nx, v1.ny, v1.nz) * one_over_w1;
    let normal2 = vec3<f32>(v2.nx, v2.ny, v2.nz) * one_over_w2;
    let normal3 = vec3<f32>(v3.nx, v3.ny, v3.nz) * one_over_w3;

    let uv1 = vec2<f32>(v1.u, v1.v) * one_over_w1;
    let uv2 = vec2<f32>(v2.u, v2.v) * one_over_w2;
    let uv3 = vec2<f32>(v3.u, v3.v) * one_over_w3;

    // Pre-divide z by w for perspective-correct depth interpolation
    let z1 = v1.z * one_over_w1;
    let z2 = v2.z * one_over_w2;
    let z3 = v3.z * one_over_w3;

    for (var x: u32 = tile_start_x; x < tile_end_x; x = x + 1u) {
        for (var y: u32 = tile_start_y; y < tile_end_y; y = y + 1u) {
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

            // Interpolate z (already divided by w)
            let interpolated_z = bc.x * z1 + bc.y * z2 + bc.z * z3;

            // Skip if depth is outside valid range (relaxed bounds)
            if interpolated_z < -1.0 || interpolated_z > 1.0 {
                continue;
            }

            // Convert to [0,1] range for depth buffer
            let depth = interpolated_z;
            let pixel_id = x + y * u32(screen_dims.width);

            // Early depth test before doing expensive interpolations
            let stored_depth = unpack_u32_to_float(atomicLoad(&fragment_buffer.frags[pixel_id].depth));
            if depth >= stored_depth {
                continue;
            }

            // Interpolate UV coordinates (already divided by w)
            let interpolated_uv = bc.x * uv1 + bc.y * uv2 + bc.z * uv3;

            // Interpolate world position (using barycentric directly since these are in world space)
            let interpolated_world_pos = bc.x * world_pos1 + bc.y * world_pos2 + bc.z * world_pos3;

            // Interpolate normal (perspective-correct)
            let interpolated_normal = normalize(bc.x * normal1 + bc.y * normal2 + bc.z * normal3);

            let packed_depth = pack_float_to_u32(depth);
            atomicStore(&fragment_buffer.frags[pixel_id].depth, packed_depth);
            fragment_buffer.frags[pixel_id].uv = interpolated_uv;
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
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;

    if tile_x >= num_tiles_x || tile_y >= num_tiles_y {
        return;
    }

    let tile_idx = tile_x + tile_y * num_tiles_x;
    let triangle_count = atomicLoad(&tile_buffer.triangle_indices[tile_idx].count);
    let triangle_offset = tile_buffer.triangle_indices[tile_idx].offset;

    // Process each triangle in this tile
    for (var i = 0u; i < triangle_count; i = i + 1u) {
        let base_idx = triangle_list_buffer.indices[triangle_offset + i];
        let idx1 = index_buffer.values[base_idx];
        let idx2 = index_buffer.values[base_idx + 1u];
        let idx3 = index_buffer.values[base_idx + 2u];

        let v1 = projected_buffer.values[idx1];
        let v2 = projected_buffer.values[idx2];
        let v3 = projected_buffer.values[idx3];

        // Only skip if all vertices are behind the near plane:
        if v1.w_clip < 0.0 || v2.w_clip < 0.0 || v3.w_clip < 0.0 {
            continue;
        }

        // Calculate triangle winding in screen space
        let a = vec2<f32>(v2.x - v1.x, v2.y - v1.y);
        let b = vec2<f32>(v3.x - v1.x, v3.y - v1.y);
        let cross_z = a.x * b.y - a.y * b.x;

        // Skip back-facing triangles only if effect doesn't require seeing inside
        // For now, we'll keep back-face culling disabled when inside objects
        if effect.effect_type != 3u && cross_z >= 0.0 {
            continue;
        }

        rasterize_triangle_in_tile(v1, v2, v3, tile_x, tile_y);
    }
}