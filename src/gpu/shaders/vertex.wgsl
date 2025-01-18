// -----------------------------------------------------------------------------
// VERTEX STAGE
// -----------------------------------------------------------------------------

struct Uniform {
    width: f32,
    height: f32,
};

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

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

struct VertexBuffer {
    values: array<Vertex>,
};

struct IndexBuffer {
    values: array<u32>,
};

struct ProjectedVertexBuffer {
    values: array<Vertex>,
};

struct EffectUniform {
    effect_type: u32,
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    time: f32,
    _padding: vec2<f32>,
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

// -----------------------------------------------------------------------------
// BINDINGS
// -----------------------------------------------------------------------------
@group(0) @binding(0) var<storage, read> vertex_buffer: VertexBuffer;
@group(0) @binding(1) var<storage, read> index_buffer: IndexBuffer;
@group(0) @binding(2) var<storage, read_write> projected_buffer: ProjectedVertexBuffer;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;
@group(3) @binding(0) var<uniform> effect: EffectUniform;

// -----------------------------------------------------------------------------
// VERTEX LOGIC
// -----------------------------------------------------------------------------

// The original projection logic, but placed in a separate function here.
fn project_vertex(v: Vertex) -> Vertex {
    var modified_v = v;
    var world_pos = vec3<f32>(v.x, v.y, v.z);

    // If there's an effect that modifies position, apply it:
    if effect.effect_type == 1u { // Wave
        world_pos = apply_wave_effect(world_pos, effect);
    }

    // Multiply by the view-projection matrix.
    let clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // Check if vertex is behind near plane (w < 0)
    // We'll mark these vertices with a special w value that the rasterizer can check
    if clip_pos.w <= 0.0 {
        return Vertex(
            0.0, 0.0, 0.0,  // Invalid screen position
            v.u, v.v,
            v.nx, v.ny, v.nz,  // Keep original normals
            v.texture_index,
            -1.0, // Special marker for invalid vertices
        );
    }

    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert NDC -> screen
    let screen_pos = vec3<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        1.0 - ndc_pos.z
    );

    // Keep the original normals and store world position separately
    return Vertex(
        screen_pos.x,
        screen_pos.y,
        screen_pos.z,
        v.u,
        v.v,
        v.nx,
        v.ny,
        v.nz,
        v.texture_index,
        clip_pos.w,
    );
}

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
@compute @workgroup_size(16, 16)
fn vertex_main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = global_id.y * num_workgroups.x * 16 + global_id.x;
    if idx >= arrayLength(&index_buffer.values) {
        return;
    }

    // Get vertex through index buffer
    let vertex_idx = index_buffer.values[idx];
    let v = vertex_buffer.values[vertex_idx];
    let projected = project_vertex(v);
    projected_buffer.values[vertex_idx] = projected;
}