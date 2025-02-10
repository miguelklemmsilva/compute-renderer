// Define a tile size constant
const TILE_SIZE: u32 = 8u;

// Data structures (unchanged)
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

struct TileTriangles {
    count: atomic<u32>,
    offset: u32,
    write_index: atomic<u32>,
    padding: u32,
};

struct TileBuffer {
    triangle_indices: array<TileTriangles>,
};

struct TriangleListBuffer {
    indices: array<u32>,
};

struct UniformBinning {
    width: f32,
    height: f32,
};

struct PartialSums {
    values: array<atomic<u32>>,
};

@group(0) @binding(0) var<storage, read> projected_buffer: ProjectedVertexBuffer;
@group(0) @binding(1) var<storage, read> index_buffer: IndexBuffer;
@group(0) @binding(2) var<storage, read_write> tile_buffer: TileBuffer;
@group(0) @binding(3) var<storage, read_write> triangle_list_buffer: TriangleListBuffer;
@group(0) @binding(4) var<storage, read_write> partial_sums: PartialSums;

@group(1) @binding(0) var<uniform> screen_dims: UniformBinning;

// Use workgroup shared memory for the local scan:
var<workgroup> shared_data: array<u32, 256>;

//---------------------------------------------------------------------
// Utility: Compute the axis–aligned bounding box of a triangle (x and y)
fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    let min_x = min(min(v1.x, v2.x), v3.x);
    let min_y = min(min(v1.y, v2.y), v3.y);
    let max_x = max(max(v1.x, v2.x), v3.x);
    let max_y = max(max(v1.y, v2.y), v3.y);
    return vec4<f32>(min_x, min_y, max_x, max_y);
}

//---------------------------------------------------------------------
// Kernel 1: Count triangles per tile.
// Each thread processes one triangle, inlining the triangle logic.
@compute @workgroup_size(1, 1, 32)
fn count_triangles(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let triangle_index = wg.x + wg.y * num_workgroups.x;
    let num_triangles = arrayLength(&index_buffer.values) / 3u;
    if triangle_index >= num_triangles {
        return;
    }
    // Get the three vertex indices and load the vertices.
    let base_idx = triangle_index * 3u;
    let idx1 = index_buffer.values[base_idx];
    let idx2 = index_buffer.values[base_idx + 1u];
    let idx3 = index_buffer.values[base_idx + 2u];
    let v1 = projected_buffer.values[idx1];
    let v2 = projected_buffer.values[idx2];
    let v3 = projected_buffer.values[idx3];

    // Discard triangles with any vertex behind the near plane.
    if v1.w_clip < 0.0 || v2.w_clip < 0.0 || v3.w_clip < 0.0 {
        return;
    }

    // Compute the 2D bounding box.
    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

    // Discard triangles completely outside the screen bounds.
    if min_max.z < 0.0 || min_max.x >= screen_dims.width || min_max.w < 0.0 || min_max.y >= screen_dims.height {
        return;
    }

    // Compute the tile range that the triangle's bounding box touches.
    let start_tile_x = u32(max(floor(min_max.x / f32(TILE_SIZE)), 0.0));
    let start_tile_y = u32(max(floor(min_max.y / f32(TILE_SIZE)), 0.0));
    let end_tile_x = min(u32(ceil(min_max.z / f32(TILE_SIZE))),
        (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE);
    let end_tile_y = min(u32(ceil(min_max.w / f32(TILE_SIZE))),
        (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE);

    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_range_x = end_tile_x - start_tile_x;
    let tile_range_y = end_tile_y - start_tile_y;
    let num_tiles = tile_range_x * tile_range_y;

    // For each tile covered by the triangle's bounding box, count its contribution.
    let num_threads = 32u; // matches workgroup size in z
    let thread_id = lid.z;
    for (var i: u32 = thread_id; i < num_tiles; i = i + num_threads) {
        let tile_x = start_tile_x + (i % tile_range_x);
        let tile_y = start_tile_y + (i / tile_range_x);
        let tile_index = tile_x + tile_y * num_tiles_x;
        atomicAdd(&tile_buffer.triangle_indices[tile_index].count, 1u);
    }
}

//---------------------------------------------------------------------
// Workgroup-based exclusive scan routine.
// This routine uses shared memory to compute prefix sums over 256 values.
fn workgroup_scan_exclusive(tid: u32, workgroup_size: u32) -> u32 {
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
    if tid == workgroup_size - 1u {
        shared_data[tid] = 0u;
    }
    d = 1u;
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

//---------------------------------------------------------------------
// Kernel 2a: First pass of scan over per–tile triangle counts.
// Each workgroup loads up to 256 tile counts, computes an exclusive scan,
// and the last thread writes the workgroup's total to the partial_sums array.
@compute @workgroup_size(256)
fn scan_first_pass(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;
    let total_tiles = num_tiles_x * num_tiles_y;

    let tile_index = global_id.x;
    let tid = local_id.x;
    shared_data[tid] = 0u;
    if tile_index < total_tiles {
        shared_data[tid] = tile_buffer.triangle_indices[tile_index].count;
    }
    workgroupBarrier();

    let scan_result = workgroup_scan_exclusive(tid, 256u);

    if tid == 255u {
        let workgroup_sum = scan_result + shared_data[tid];
        atomicStore(&partial_sums.values[workgroup_id.x], workgroup_sum);
    }
    storageBarrier();

    if tile_index < total_tiles {
        tile_buffer.triangle_indices[tile_index].offset = scan_result;
    }
}

//---------------------------------------------------------------------
// Kernel 2b: Second pass for scan.
// Each thread adds the sum of all previous workgroups to its tile offset.
@compute @workgroup_size(256)
fn scan_second_pass(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE;
    let total_tiles = num_tiles_x * num_tiles_y;
    let tile_index = global_id.x;
    if tile_index >= total_tiles {
        return;
    }
    
    var workgroup_offset = 0u;
    let current_group = workgroup_id.x;
    for (var i = 0u; i < current_group; i = i + 1u) {
        workgroup_offset += atomicLoad(&partial_sums.values[i]);
    }
    tile_buffer.triangle_indices[tile_index].offset += workgroup_offset;
}

//---------------------------------------------------------------------
// Kernel 3: Store triangle indices into the triangle list buffer.
// Each thread processes one triangle and inlines the triangle logic.
@compute @workgroup_size(1, 1, 32)
fn store_triangles(
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    // Compute the global triangle index from the 2D workgroup grid.
    let triangle_index = wg.x + wg.y * num_workgroups.x;
    let num_triangles = arrayLength(&index_buffer.values) / 3u;
    if triangle_index >= num_triangles {
        return;
    }
    
    // The rest of the kernel remains similar to your original logic:
    let base_idx = triangle_index * 3u;
    let idx1 = index_buffer.values[base_idx];
    let idx2 = index_buffer.values[base_idx + 1u];
    let idx3 = index_buffer.values[base_idx + 2u];
    let v1 = projected_buffer.values[idx1];
    let v2 = projected_buffer.values[idx2];
    let v3 = projected_buffer.values[idx3];

    // Discard triangles that are off-screen or behind the near plane.
    if v1.w_clip < 0.0 || v2.w_clip < 0.0 || v3.w_clip < 0.0 {
        return;
    }

    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );
    if min_max.z < 0.0 || min_max.x >= screen_dims.width || min_max.w < 0.0 || min_max.y >= screen_dims.height {
        return;
    }

    let start_tile_x = u32(max(floor(min_max.x / f32(TILE_SIZE)), 0.0));
    let start_tile_y = u32(max(floor(min_max.y / f32(TILE_SIZE)), 0.0));
    let end_tile_x = min(u32(ceil(min_max.z / f32(TILE_SIZE))),
        (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE);
    let end_tile_y = min(u32(ceil(min_max.w / f32(TILE_SIZE))),
        (u32(screen_dims.height) + TILE_SIZE - 1u) / TILE_SIZE);

    let num_tiles_x = (u32(screen_dims.width) + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_range_x = end_tile_x - start_tile_x;
    let tile_range_y = end_tile_y - start_tile_y;
    let total_tiles = tile_range_x * tile_range_y;

    let num_threads = 32u; // matches workgroup size in z
    let thread_id = lid.z;
    for (var i: u32 = thread_id; i < total_tiles; i = i + num_threads) {
        let tile_x = start_tile_x + (i % tile_range_x);
        let tile_y = start_tile_y + (i / tile_range_x);
        let tile_index = tile_x + tile_y * num_tiles_x;

        let count = atomicLoad(&tile_buffer.triangle_indices[tile_index].count);
        let write_index = atomicAdd(&tile_buffer.triangle_indices[tile_index].write_index, 1u);
        if write_index < count {
            let offset = tile_buffer.triangle_indices[tile_index].offset;
            triangle_list_buffer.indices[offset + write_index] = base_idx;
        } else {
            atomicSub(&tile_buffer.triangle_indices[tile_index].count, 1u);
        }
    }
}