struct CameraUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Light {
    world_position: vec3<f32>,
    _pad1: f32,
    view_position: vec3<f32>,
    _pad2: f32,
    color: vec3<f32>,
    intensity: f32,
};

struct ScreenUniform {
    width: f32,
    height: f32,
}

@group(0) @binding(0) var<storage, read> output_buffer: array<u32>;
@group(1) @binding(0) var<uniform> screen_dims : ScreenUniform;

// Vertex outputs
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn vs_main_trig(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    let uv = vec2<u32>((vertex_idx << 1u) & 2u, vertex_idx & 2u);
    let out = VertexOutput(vec4<f32>(1.0 - 2.0 * vec2<f32>(uv), 0.0, 1.0));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = floor(in.pos.x);
    let y = floor(in.pos.y);
    let index = u32(x + y * screen_dims.width);
    let p = output_buffer[index];
    return vec4<f32>(f32(p & 0xFF) / 255.0, f32((p >> 8) & 0xFF) / 255.0, f32((p >> 16) & 0xFF) / 255.0, 1.0);
}