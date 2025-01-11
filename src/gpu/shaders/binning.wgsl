const TILE_SIZE: u32 = 8u;

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    u: f32,
    v: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    texture_index: u32,
    w_clip: f32,
};

struct ProjectedVertexBuffer {
    values: array<Vertex>,
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

struct UniformBinning {
    width: f32,
    height: f32,
}

@group(0) @binding(0) var<storage, read> projected_buffer: ProjectedVertexBuffer;
@group(0) @binding(1) var<storage, read_write> tile_buffer: TileBuffer;
@group(0) @binding(2) var<storage, read_write> triangle_list_buffer: TriangleListBuffer;

@group(1) @binding(0) var<uniform> screen_dims: UniformBinning;

fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    var min_max = vec4<f32>();
    min_max.x = min(min(v1.x, v2.x), v3.x);
    min_max.y = min(min(v1.y, v2.y), v3.y);
    min_max.z = max(max(v1.x, v2.x), v3.x);
    min_max.w = max(max(v1.y, v2.y), v3.y);
    return min_max;
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

fn process_triangle(triangle_index: u32, count_only: bool) {
    let num_triangles = arrayLength(&projected_buffer.values);
    
    // Early exit if this thread is beyond the number of triangles
    if triangle_index >= num_triangles {
        return;
    }

    // Get the three vertices of this triangle
    let v1 = projected_buffer.values[triangle_index];
    let v2 = projected_buffer.values[triangle_index + 1u];
    let v3 = projected_buffer.values[triangle_index + 2u];

    // Skip triangles with any vertices behind the near plane
    if v1.w_clip < 0.0 || v2.w_clip < 0.0 || v3.w_clip < 0.0 {
        return;
    }

    // Calculate triangle bounding box
    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

    // Skip triangles completely outside NDC space
    if min_max.z < 0.0 || min_max.x > screen_dims.width || min_max.w < 0.0 || min_max.y > screen_dims.height {
        return;
    }

    // Calculate which tiles this triangle might overlap
    let start_tile_x = u32(max(floor(min_max.x / f32(TILE_SIZE)), 0.0));
    let start_tile_y = u32(max(floor(min_max.y / f32(TILE_SIZE)), 0.0));
    let end_tile_x = min(u32(ceil(min_max.z / f32(TILE_SIZE))), (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE);
    let end_tile_y = min(u32(ceil(min_max.w / f32(TILE_SIZE))), (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE);

    // For each tile that this triangle might overlap
    for (var tile_y = start_tile_y; tile_y <= end_tile_y; tile_y = tile_y + 1u) {
        for (var tile_x = start_tile_x; tile_x <= end_tile_x; tile_x = tile_x + 1u) {
            // Calculate the tile index
            let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
            let tile_index = tile_x + tile_y * num_tiles_x;

            // If the triangle actually overlaps this tile
            if triangle_overlaps_tile(min_max, tile_x, tile_y) {
                if count_only {
                    // First pass: just count triangles
                    atomicAdd(&tile_buffer.triangle_indices[tile_index].count, 1u);
                } else {
                    // Second pass: store triangle indices
                    let offset = tile_buffer.triangle_indices[tile_index].offset;
                    let write_index = atomicAdd(&tile_buffer.triangle_indices[tile_index].write_index, 1u);
                    // Add bounds check to prevent buffer overflow
                    let max_triangles = arrayLength(&triangle_list_buffer.indices);
                    if (offset + write_index) < max_triangles {
                        triangle_list_buffer.indices[offset + write_index] = triangle_index;
                    } else {
                        atomicSub(&tile_buffer.triangle_indices[tile_index].count, 1u);
                    }
                }
            }
        }
    }
}

@compute @workgroup_size(16, 16)
fn count_triangles(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = global_id.y * num_workgroups.x * 16 + global_id.x;
    process_triangle(idx * 3u, true);
}

// New function to calculate offsets based on counts
@compute @workgroup_size(16, 16)
fn calculate_offsets(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tile_x = global_id.x;
    let tile_y = global_id.y;
    // Early exit if this tile is outside the screen
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;

    if tile_x >= num_tiles_x || tile_y >= num_tiles_y {
        return;
    }

    let tile_idx = tile_x + tile_y * num_tiles_x;

    // First, calculate prefix sum of counts up to this tile
    var offset = 0u;
    for (var i = 0u; i < tile_idx; i = i + 1u) {
        offset = offset + tile_buffer.triangle_indices[i].count;
    }
    
    // Store the calculated offset
    tile_buffer.triangle_indices[tile_idx].offset = offset;
}

@compute @workgroup_size(16, 16)
fn store_triangles(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = global_id.y * num_workgroups.x * 16 + global_id.x;
    process_triangle(idx * 3u, false);
}
