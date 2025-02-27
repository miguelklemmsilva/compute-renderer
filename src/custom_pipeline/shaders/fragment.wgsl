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


@group(0) @binding(0) var<storage, read_write> output_buffer: array<u32>;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read> lights: array<Light>;

@group(4) @binding(0) var<uniform> effect: EffectUniform;

// The fragment data & count from the raster pass
@group(5) @binding(0) var<storage, read_write> fragment_buffer: array<Fragment>;

@compute @workgroup_size(256)
fn fragment_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x + global_id.y * u32(screen_dims.width);
    output_buffer[idx] = 0u;

    // Early-out if there's no valid fragment
    if idx >= arrayLength(&fragment_buffer) || fragment_buffer[idx].flag == 0u {
        return;
    }

    let in = fragment_buffer[idx];

    var normal = in.normal;
    
    var final_color = vec3<f32>(0.3);

    let num_lights = arrayLength(&lights);
    for (var i = 0u; i < num_lights; i++) {
        let light = lights[i];
        let light_dir = normalize(light.world_position - in.position);
        let diff = max(dot(normal, light_dir), 0.0);
        let view_dir = normalize(camera.view_pos.xyz - in.position);
        let reflect_dir = reflect(-light_dir, normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        final_color += (diff + spec * 0.5) * light.color * light.intensity;
    }

    fragment_buffer[idx].flag = 0u;

    final_color = clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0));

    let srgb_color = pow(final_color, vec3<f32>(1.0 / 2.2));

    // Convert float color to integer
    let R = u32(srgb_color.x * 255.0);
    let G = u32(srgb_color.y * 255.0);
    let B = u32(srgb_color.z * 255.0);

    let output_color = (255u << 24u) | (B << 16u) | (G << 8u) | R;
    output_buffer[idx] = output_color;
}