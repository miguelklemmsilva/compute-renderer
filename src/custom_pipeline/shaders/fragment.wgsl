struct Uniform {
    width: f32,
    height: f32,
};

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Light {
    world_position: vec3<f32>,
    _padding1: f32,
    view_position: vec3<f32>,
    _padding2: f32,
    color: vec3<f32>,
    intensity: f32,
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

struct Fragment {
    uv: vec2<f32>,
    normal: vec3<f32>,
    position: vec3<f32>,
    flag: u32
};


@group(0) @binding(0) var output_tex: texture_storage_2d<bgra8unorm, write>;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read> lights: array<Light>;

@group(4) @binding(0) var<uniform> effect: EffectUniform;

// The fragment data & count from the raster pass
@group(5) @binding(0) var<storage, read_write> fragment_buffer: array<Fragment>;

@compute @workgroup_size(256)
fn fragment_main(@builtin(global_invocation_id) global_id: vec3<u32>) {

    let idx = global_id.x;
    let tex_width = u32(screen_dims.width);  // Use the actual texture width
    let x = i32(idx % tex_width);
    let y = i32(idx / tex_width);

    textureStore(
        output_tex,
        vec2<i32>(x, y),
        vec4<f32>(0.0, 0.0, 1.0, 1.0)
    );

    // Early-out if there's no valid fragment
    if idx >= arrayLength(&fragment_buffer) || fragment_buffer[idx].flag == 0u {
        return;
    }


    let in = fragment_buffer[idx];
    var final_color = vec3<f32>(0.1);

    let num_lights = arrayLength(&lights);
    for (var i = 0u; i < num_lights; i++) {
        let light = lights[i];
        let light_dir = normalize(light.world_position - in.position);
        let diff = max(dot(in.normal, light_dir), 0.0);
        let view_dir = normalize(camera.view_pos.xyz - in.position);
        let reflect_dir = reflect(-light_dir, in.normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        final_color += (diff + spec * 0.5) * light.color * light.intensity;
    }

    fragment_buffer[idx].flag = 0u;
    final_color = clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0));
    let srgb_color = pow(final_color, vec3<f32>(1.0 / 2.2));

    textureStore(
        output_tex,
        vec2<i32>(x, y),
        vec4<f32>(srgb_color, 1.0)
    );
}