const TILE_SIZE: u32 = 8u;
const MAX_TRIANGLES_PER_TILE: u32 = 1024u;

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
    triangle_indices: array<u32, MAX_TRIANGLES_PER_TILE>,
};

struct TileBuffer {
    triangle_indices: array<TileTriangles>,
}

struct UniformBinning {
    width: f32,
    height: f32,
}

@group(0) @binding(0) var<storage, read> projected_buffer: ProjectedVertexBuffer;
@group(0) @binding(1) var<storage, read_write> tile_buffer: TileBuffer;

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

@compute @workgroup_size(256)
fn bin_triangles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_index = global_id.x * 3u;
    let num_triangles = arrayLength(&projected_buffer.values);
    
    // Early exit if this thread is beyond the number of triangles
    if triangle_index >= num_triangles {
        return;
    }

    // Get the three vertices of this triangle
    let v1 = projected_buffer.values[triangle_index];
    let v2 = projected_buffer.values[triangle_index + 1u];
    let v3 = projected_buffer.values[triangle_index + 2u];

    // Calculate triangle bounding box
    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

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
                // Add the triangle to the tile's list
                let new_count = atomicAdd(&tile_buffer.triangle_indices[tile_index].count, 1u);
                // Only add if we haven't exceeded the maximum
                if new_count < MAX_TRIANGLES_PER_TILE {
                    tile_buffer.triangle_indices[tile_index].triangle_indices[new_count] = triangle_index;
                } else {
                    atomicSub(&tile_buffer.triangle_indices[tile_index].count, 1u);
                }
            }
        }
    }
}