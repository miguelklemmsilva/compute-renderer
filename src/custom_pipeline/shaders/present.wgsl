struct ScreenUniform {
    width: f32,
    height: f32,
};

@group(0) @binding(0)
var my_tex: texture_2d<f32>;

@group(0) @binding(1)
var my_sampler: sampler;

@group(1) @binding(0)
var<uniform> screen_dims: ScreenUniform;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32)
    -> @builtin(position) vec4<f32> {

    // Draw a single full-screen triangle
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32((vertex_index & 2u));
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(my_tex, 0));
    let uv = coord.xy / vec2<f32>(screen_dims.width, screen_dims.height);
    return textureSample(my_tex, my_sampler, uv);
}
