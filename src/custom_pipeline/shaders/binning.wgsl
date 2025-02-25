// Define a tile size constant
const TILE_SIZE = 8u;

struct Vertex {
    world_pos: vec3<f32>,
    screen_pos: vec4<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
};

struct TileTriangles {
    count: atomic<u32>,
    offset: u32,
    write_index: atomic<u32>,
};

struct UniformBinning {
    width: f32,
    height: f32,
    num_tiles_x: u32,
    num_tiles_y: u32,
};

// Precomputed metadata for each triangle.
struct TriangleBinningData {
    // Screen-space bounding box: (min_x, min_y, max_x, max_y)
    min_max: vec4<f32>,
    // Tile in which the triangle starts.
    start_tile: vec2<u32>,
    // Number of tiles covered in x and y.
    tile_range: vec2<u32>,
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

struct VertexIn {
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
};

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

// universal buffers
@group(0) @binding(0) var<storage, read_write> tile_buffer: array<TileTriangles>;
@group(0) @binding(1) var<storage, read_write> triangle_binning_buffer: array<TriangleBinningData>;
@group(0) @binding(2) var<uniform> screen_dims: UniformBinning;
@group(0) @binding(3) var<uniform> effect: EffectUniform;

@group(1) @binding(0) var<storage, read_write> partial_sums: array<u32>;

@group(2) @binding(0) var<storage, read> index_buffer: array<u32>;
@group(2) @binding(1) var<storage, read> vertex_buffer: array<VertexIn>;
@group(2) @binding(2) var<storage, read_write> projected_buffer: array<Vertex>;
@group(2) @binding(3) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read_write> triangle_list_buffer: array<u32>;


// Use workgroup shared memory for the local scan:
var<workgroup> shared_data: array<u32, 256u>;

fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    let min_x = min(min(v1.x, v2.x), v3.x);
    let min_y = min(min(v1.y, v2.y), v3.y);
    let max_x = max(max(v1.x, v2.x), v3.x);
    let max_y = max(max(v1.y, v2.y), v3.y);
    return vec4<f32>(min_x, min_y, max_x, max_y);
}

// Helper function to clip a bounding box to the screen (frustum) bounds.
fn clip_bbox_to_screen(bbox: vec4<f32>) -> vec4<f32> {
    // bbox = (min_x, min_y, max_x, max_y)
    return vec4<f32>(
        max(bbox.x, 0.0),
        max(bbox.y, 0.0),
        min(bbox.z, screen_dims.width - 1.0),
        min(bbox.w, screen_dims.height - 1.0)
    );
}

fn compute_triangle_meta(triangle_index: u32) {
    let v1 = shared_v[0];
    let v2 = shared_v[1];
    let v3 = shared_v[2];
    
    // First, perform a simple clip test in clip/screen space:
    // Discard triangles with any vertex behind the near plane.
    if v1.screen_pos.w < 0.0 || v2.screen_pos.w < 0.0 || v3.screen_pos.w < 0.0 {
        triangle_binning_buffer[triangle_index].tile_range = vec2<u32>(0u, 0u);
        return;
    }
    
    // Compute the 2D bounding box in screen space.
    let bbox = get_min_max(v1.screen_pos.xyz, v2.screen_pos.xyz, v3.screen_pos.xyz);
    
    // Quick cull: if the triangle’s bbox is completely outside the screen,
    // then discard it.
    if bbox.z < 0.0 || bbox.x >= screen_dims.width || bbox.w < 0.0 || bbox.y >= screen_dims.height {
        triangle_binning_buffer[triangle_index].tile_range = vec2<u32>(0u, 0u);
        return;
    }

    // Back-face culling (unless the effect requires both sides).
    let a = vec2<f32>(v2.screen_pos.x - v1.screen_pos.x, v2.screen_pos.y - v1.screen_pos.y);
    let b = vec2<f32>(v3.screen_pos.x - v1.screen_pos.x, v3.screen_pos.y - v1.screen_pos.y);
    let cross_z = a.x * b.y - a.y * b.x;
    if effect.effect_type != 3u && cross_z >= 0.0 {
        triangle_binning_buffer[triangle_index].tile_range = vec2<u32>(0u, 0u);
        return;
    }
    
    // Now “clip” the bbox to the screen dimensions.
    let clipped_bbox = clip_bbox_to_screen(bbox);
    
    // Use the clipped bbox to compute tile indices.
    let start_tile_x = u32(max(floor(clipped_bbox.x / f32(TILE_SIZE)), 0.0));
    let start_tile_y = u32(max(floor(clipped_bbox.y / f32(TILE_SIZE)), 0.0));
    let end_tile_x = min(u32(ceil(clipped_bbox.z / f32(TILE_SIZE))),
        screen_dims.num_tiles_x);
    let end_tile_y = min(u32(ceil(clipped_bbox.w / f32(TILE_SIZE))),
        screen_dims.num_tiles_y);

    let tile_range_x = end_tile_x - start_tile_x;
    let tile_range_y = end_tile_y - start_tile_y;
    
    // Store the computed metadata.
    triangle_binning_buffer[triangle_index].min_max = clipped_bbox;
    triangle_binning_buffer[triangle_index].start_tile = vec2<u32>(start_tile_x, start_tile_y);
    triangle_binning_buffer[triangle_index].tile_range = vec2<u32>(tile_range_x, tile_range_y);
}

fn apply_wave_effect(pos: vec3<f32>, effect: EffectUniform) -> vec3<f32> {
    var modified_pos = pos;
    let amplitude = effect.param1;
    let frequency = effect.param2;
    let phase = effect.param3;
    let direction = effect.param4;

    if direction < 0.5 { // Vertical
        modified_pos.y += amplitude * sin(frequency * pos.x + phase);
    } else if direction < 1.5 { // Horizontal
        modified_pos.x += amplitude * sin(frequency * pos.y + phase);
    } else { // Radial
        let dist = length(pos.xy);
        modified_pos.z += amplitude * sin(frequency * dist + phase);
    }

    return modified_pos;
}

fn compute_screen_pos(clip_pos: vec4<f32>) -> vec4<f32> {
    let ndc_pos = clip_pos.xyz / clip_pos.w;

    return vec4<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        clip_pos.z / clip_pos.w,
        clip_pos.w
    );
}

fn geometry_pipeline(idx: u32) -> Vertex {
    // Load original vertex data.
    let v_in = vertex_buffer[idx];

    // Apply any effects if needed.
    var world_pos = v_in.world_pos;
    if effect.effect_type == 1u {
        world_pos = apply_wave_effect(world_pos, effect);
    }
        
    // Transform to clip space and then compute screen positions.
    let clip = camera.view_proj * vec4<f32>(world_pos, 1.0);
    let screen_pos = compute_screen_pos(clip);

    return Vertex(world_pos, screen_pos, v_in.normal, v_in.uv);
}

// Use workgroup shared memory for the transformed vertices.
var<workgroup> shared_v: array<Vertex, 3>;

const Z_DISPATCHES = 3u;
@compute @workgroup_size(1, 1, Z_DISPATCHES)
fn count_triangles(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let triangle_index = wg.x + wg.y * num_workgroups.x;

    let num_triangles = arrayLength(&triangle_binning_buffer);
    if triangle_index >= num_triangles {
        return;
    }

    let thread_id = lid.z;

    // Each workgroup handles one triangle.
    let base_idx = triangle_index * 3u;

    let idx1 = index_buffer[base_idx];
    let idx2 = index_buffer[base_idx + 1u];
    let idx3 = index_buffer[base_idx + 2u];

    var vertex = geometry_pipeline(index_buffer[base_idx + lid.z]);

    shared_v[lid.z] = vertex;
    projected_buffer[index_buffer[base_idx + lid.z]] = vertex;

    // 1) Compute metadata for this triangle
    compute_triangle_meta(triangle_index);
    let triangle_meta = triangle_binning_buffer[triangle_index];

    let start_tile_x = triangle_meta.start_tile.x;
    let start_tile_y = triangle_meta.start_tile.y;
    let tile_range_x = triangle_meta.tile_range.x;
    let tile_range_y = triangle_meta.tile_range.y;
    let num_tiles = tile_range_x * tile_range_y;

    // If there's nothing to bin, exit early
    if num_tiles == 0u {
        return;
    }

    // 2) Each thread will loop over some subset of tiles
    let num_tiles_x = screen_dims.num_tiles_x;

    for (var ty = 0u; ty < tile_range_y; ty++) {
        let tile_y = start_tile_y + ty;
        for (var tx = thread_id; tx < tile_range_x; tx += Z_DISPATCHES) {
            let tile_x = start_tile_x + tx;
            let tile_index = tile_x + tile_y * num_tiles_x;
            atomicAdd(&tile_buffer[tile_index].count, 1u);
        }
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
        d >>= 1u;
    }
    if tid == workgroup_size - 1u {
        shared_data[tid] = 0u;
    }
    d = 1u;
    while d < workgroup_size {
        offset >>= 1u;
        workgroupBarrier();
        if tid < d {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            let temp = shared_data[ai];
            shared_data[ai] = shared_data[bi];
            shared_data[bi] += temp;
        }
        d <<= 1u;
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
    let num_tiles_x = screen_dims.num_tiles_x;
    let num_tiles_y = screen_dims.num_tiles_y;
    let total_tiles = num_tiles_x * num_tiles_y;

    let tile_index = global_id.x;

    if tile_index >= total_tiles {
        return;
    }

    let tid = local_id.x;

    shared_data[tid] = tile_buffer[tile_index].count;

    let scan_result = workgroup_scan_exclusive(tid, 256u);

    if tid == 255u {
        let workgroup_sum = scan_result + shared_data[tid];
        partial_sums[workgroup_id.x] = workgroup_sum;
    }

    tile_buffer[tile_index].offset = scan_result;
}

//---------------------------------------------------------------------
// Kernel 2b: Second pass for scan.
// Each thread adds the sum of all previous workgroups to its tile offset.
@compute @workgroup_size(256)
fn scan_second_pass(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let num_tiles_x = screen_dims.num_tiles_x;
    let num_tiles_y = screen_dims.num_tiles_y;
    let total_tiles = num_tiles_x * num_tiles_y;
    let tile_index = global_id.x;
    if tile_index >= total_tiles {
        return;
    }

    var workgroup_offset = 0u;
    let current_group = workgroup_id.x;
    for (var i = 0u; i < current_group; i++) {
        workgroup_offset += partial_sums[i];
    }
    tile_buffer[tile_index].offset += workgroup_offset;
}

//---------------------------------------------------------------------
// Kernel 3: Store triangle indices into the triangle list buffer.
// Each thread processes one triangle and inlines the triangle logic.
@compute @workgroup_size(1, 1, Z_DISPATCHES)
fn store_triangles(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) wg: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    // Identify which triangle this workgroup processes.
    let triangle_index = wg.x + wg.y * num_workgroups.x;
    let num_triangles = arrayLength(&triangle_binning_buffer);

    // Early out: nothing to do if out of range.
    if triangle_index >= num_triangles {
        return;
    }

    // Pull precomputed metadata:
    let triangle_meta = triangle_binning_buffer[triangle_index];
    let start_tile_x = triangle_meta.start_tile.x;
    let start_tile_y = triangle_meta.start_tile.y;
    let tile_range_x = triangle_meta.tile_range.x;
    let tile_range_y = triangle_meta.tile_range.y;
    let total_tiles = tile_range_x * tile_range_y;

    // The base index of this triangle in index_buffer
    let base_idx = triangle_index * 3u;

    // We'll again split the tile iteration among 64 threads (in z).
    let thread_id = lid.z;
    let num_tiles_x = screen_dims.num_tiles_x;
    for (var ty = 0u; ty < tile_range_y; ty ++) {
        let tile_y = start_tile_y + ty;
        for (var tx = thread_id; tx < tile_range_x; tx += Z_DISPATCHES) {
            let tile_x = start_tile_x + tx;
            let tile_index = tile_x + tile_y * num_tiles_x;

            let count = tile_buffer[tile_index].count;
            let write_index = atomicAdd(&tile_buffer[tile_index].write_index, 1u);

            triangle_list_buffer[tile_buffer[tile_index].offset + write_index] = base_idx;
        }
    }
}