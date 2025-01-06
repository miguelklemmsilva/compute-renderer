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

struct ProjectedVertexBuffer {
    values: array<Vertex>,
};

// The same effect uniform.
struct EffectUniform {
    effect_type: u32,
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    time: f32,
    _padding: vec2<f32>,
}

// -- Effects and other helper functions (exactly as before, but cut down
//    or re-copied so the logic stays the same). You can keep all the code
//    from your original file—hash, noise, wave, voxelize, etc. This snippet
//    just shows the main ones you likely need for the vertex pass.

fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
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

fn apply_voxelize_effect(pos: vec3<f32>, effect: EffectUniform) -> vec3<f32> {
    let grid_size = effect.param1;
    return floor(pos * grid_size) / grid_size;
}

// -----------------------------------------------------------------------------
// BINDINGS
// -----------------------------------------------------------------------------
@group(0) @binding(0) var<storage, read> vertex_buffer: VertexBuffer;
@group(0) @binding(1) var<storage, read_write> projected_buffer: ProjectedVertexBuffer;

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
    } else if effect.effect_type == 5u { // Voxelize
        world_pos = apply_voxelize_effect(world_pos, effect);
    }

    // Multiply by the view-projection matrix.
    let clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);
    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert NDC -> screen
    let screen_pos = vec3<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        ndc_pos.z
    );

    // Reuse normal fields to store the original world pos
    return Vertex(
        screen_pos.x,
        screen_pos.y,
        screen_pos.z,
        v.u,
        v.v,
        world_pos.x,
        world_pos.y,
        world_pos.z,
        v.texture_index,
        clip_pos.w
    );
}

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
@compute @workgroup_size(256)
fn vertex_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&vertex_buffer.values) {
        return;
    }

    let v = vertex_buffer.values[idx];
    let projected = project_vertex(v);

    // Write to a second buffer so the next pass can read
    projected_buffer.values[idx] = projected;
}