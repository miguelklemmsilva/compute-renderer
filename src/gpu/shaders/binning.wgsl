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

struct PartialSums {
    values: array<atomic<u32>>,
}

@group(0) @binding(0) var<storage, read> projected_buffer: ProjectedVertexBuffer;
@group(0) @binding(1) var<storage, read_write> tile_buffer: TileBuffer;
@group(0) @binding(2) var<storage, read_write> triangle_list_buffer: TriangleListBuffer;
@group(0) @binding(3) var<storage, read_write> partial_sums: PartialSums;

@group(1) @binding(0) var<uniform> screen_dims: UniformBinning;

// Workgroup shared memory for parallel scan
var<workgroup> shared_data: array<u32, 256>;  

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
    if min_max.z < 0.0 || min_max.x >= screen_dims.width || min_max.w < 0.0 || min_max.y >= screen_dims.height {
        return;
    }

    // Calculate which tiles this triangle might overlap
    let start_tile_x = u32(max(floor(min_max.x / f32(TILE_SIZE)), 0.0));
    let start_tile_y = u32(max(floor(min_max.y / f32(TILE_SIZE)), 0.0));
    let end_tile_x = min(u32(ceil(min_max.z / f32(TILE_SIZE))), (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE);
    let end_tile_y = min(u32(ceil(min_max.w / f32(TILE_SIZE))), (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE);

    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    
    // For each tile that this triangle might overlap
    for (var tile_y = start_tile_y; tile_y < end_tile_y; tile_y = tile_y + 1u) {
        for (var tile_x = start_tile_x; tile_x < end_tile_x; tile_x = tile_x + 1u) {
            // Calculate the tile index
            let tile_index = tile_x + tile_y * num_tiles_x;

            // If the triangle actually overlaps this tile
            if triangle_overlaps_tile(min_max, tile_x, tile_y) {
                if count_only {
                    // First pass: just count triangles
                    atomicAdd(&tile_buffer.triangle_indices[tile_index].count, 1u);
                } else {
                    // Second pass: store triangle indices
                    let count = atomicLoad(&tile_buffer.triangle_indices[tile_index].count);
                    let write_index = atomicAdd(&tile_buffer.triangle_indices[tile_index].write_index, 1u);
                    
                    // Only write if we haven't exceeded the count from the first pass
                    if write_index < count {
                        let offset = tile_buffer.triangle_indices[tile_index].offset;
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

// Parallel scan helper functions
fn workgroup_scan_exclusive(tid: u32, workgroup_size: u32) -> u32 {
    if tid >= workgroup_size {
        return 0u;
    }

    // Up-sweep phase
    var offset = 1u;
    var d = workgroup_size >> 1u;

    while d > 0u {
        workgroupBarrier();
        if tid < d {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            shared_data[bi] += shared_data[ai];
        }
        offset *= 2u;
        d = d >> 1u;
    }

    // Down-sweep phase
    d = 1u;
    offset = workgroup_size;

    while d < workgroup_size {
        offset = offset >> 1u;
        workgroupBarrier();
        if tid < d {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            let temp = shared_data[ai];
            shared_data[ai] = shared_data[bi];
            shared_data[bi] += temp;
        }
        d = d << 1u;
    }

    workgroupBarrier();
    return shared_data[tid];
}

// First pass: compute partial sums for each workgroup
@compute @workgroup_size(256)
fn scan_first_pass(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;
    let total_tiles = num_tiles_x * num_tiles_y;

    let tile_index = global_id.x;
    
    // Initialize shared memory
    shared_data[local_index] = 0u;
    // if tile_index < total_tiles {
        shared_data[local_index] = tile_buffer.triangle_indices[tile_index].count;
    // }

    workgroupBarrier();
    
    // Perform parallel scan within workgroup
    let scan_result = workgroup_scan_exclusive(local_index, 256u);
    
    // Last thread in workgroup writes total sum to partial_sums
    if local_index == 255u {
        var last_value = 0u;
        if tile_index < total_tiles {
            last_value = tile_buffer.triangle_indices[tile_index].count;
        }
        let workgroup_sum = shared_data[255u] + last_value;
        atomicStore(&partial_sums.values[workgroup_id.x], workgroup_sum);
        // partial_sums.values[workgroup_id.x + workgroup_id.y * num_workgroups.x] = workgroup_sum;
    }

    storageBarrier();
    
    // Write local scan results
    // if tile_index < total_tiles {
        tile_buffer.triangle_indices[tile_index].offset = scan_result;
    // }
}

// Second pass: scan partial sums and update final offsets
@compute @workgroup_size(256)
fn scan_second_pass(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;
    let total_tiles = num_tiles_x * num_tiles_y;

    let tile_index = global_id.x;
    if tile_index >= total_tiles {
        return;
    }

    var workgroup_offset = 0u;
    let current_workgroup = workgroup_id.x;
    
    // Sum up all previous workgroup sums more efficiently
    for (var i = 0u; i < current_workgroup; i = i + 1u) {
        let sum = atomicLoad(&partial_sums.values[i]);
        workgroup_offset += sum;
    }
    
    // Add workgroup offset to local offset
    tile_buffer.triangle_indices[tile_index].offset += workgroup_offset;
}

@compute @workgroup_size(16, 16)
fn store_triangles(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = global_id.y * num_workgroups.x * 16 + global_id.x;
    process_triangle(idx * 3u, false);
}