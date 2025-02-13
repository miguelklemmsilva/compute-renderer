struct Uniform {
    width: f32,
    height: f32,
};

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Vertex {
    world_pos: vec4<f32>,
    normal: vec4<f32>,
    uv: vec2<f32>,
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
@group(0) @binding(0) var<storage, read> vertex_buffer: array<Vertex>;
@group(0) @binding(1) var<storage, read> index_buffer: array<u32>;
@group(0) @binding(2) var<storage, read_write> projected_buffer: array<Vertex>;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;
@group(3) @binding(0) var<uniform> effect: EffectUniform;

// -----------------------------------------------------------------------------
// VERTEX LOGIC
// -----------------------------------------------------------------------------

// The original projection logic, but placed in a separate function here.
fn project_vertex(v: Vertex) -> Vertex {
    var modified_v = v;
    var world_pos = v.world_pos;

    // If there's an effect that modifies position, apply it:
    if effect.effect_type == 1u { // Wave
        world_pos = vec4<f32>(apply_wave_effect(world_pos.xyz, effect), 1.0);
    }

    // Multiply by the view-projection matrix.
    let clip_pos = camera.view_proj * world_pos;

    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert NDC -> screen
    let screen_pos = vec4<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        clip_pos.z,
        clip_pos.w
    );

    // Keep the original normals and store world position separately
    return Vertex(
        screen_pos,
        v.normal,
        v.uv
    );
}

@compute @workgroup_size(256)
fn vertex_main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&index_buffer) {
        return;
    }

    // Get vertex through index buffer
    let vertex_idx = index_buffer[idx];
    let v = vertex_buffer[vertex_idx];
    let projected = project_vertex(v);
    projected_buffer[vertex_idx] = projected;
}