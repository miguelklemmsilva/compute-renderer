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
    world_pos: vec4<f32>,
};


@group(0) @binding(0) var<storage, read_write> output_buffer: array<u32>;
@group(0) @binding(1) var <storage, read_write> depth_buffer: array<u32>;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read> light_buffer: array<Light>;

@group(4) @binding(0) var<uniform> effect: EffectUniform;

// The fragment data & count from the raster pass
@group(5) @binding(0) var<storage, read> fragment_buffer: array<Fragment>;

fn rgba(r: u32, g: u32, b: u32, a: u32) -> u32 {
    // BGRA format (0xFF for alpha)
    return (a << 24u) | (b << 16u) | (g << 8u) | r;
}

fn calculate_lighting(normal: vec3<f32>, position: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    // Basic lighting calculation
    var final_color = vec3<f32>(0.0);
    let ambient = vec3<f32>(0.1);
    
    // Start with ambient light
    final_color = ambient;

    let num_lights = arrayLength(&light_buffer);
    
    // Add contribution from each light
    for (var i = 0u; i < num_lights; i++) {
        let light = light_buffer[i];
        let light_dir = normalize(light.world_position - position);
        
        // Diffuse
        let diff = max(dot(normal, light_dir), 0.0);
        
        // Specular
        let view_dir = normalize(camera.view_pos.xyz - position);
        let reflect_dir = reflect(-light_dir, normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);

        final_color += (diff + spec * 0.5) * light.color * light.intensity;
    }
    
    // Ensure the color doesn't exceed 1.0
    final_color = min(final_color, vec3<f32>(1.0));

    return final_color;
}

@compute @workgroup_size(256)
fn fragment_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x + global_id.y * u32(screen_dims.width);
    output_buffer[idx] = 0u;

    // Early-out if there's no valid fragment
    if idx >= arrayLength(&fragment_buffer) || depth_buffer[idx] >= 0xFFFFFFFFu {
        return;
    }

    var normal = normalize(fragment_buffer[idx].normal);

    // 3) Calculate lighting with the material
    let lighting_color = calculate_lighting(normal.xyz, fragment_buffer[idx].world_pos.xyz, fragment_buffer[idx].uv);

    // 6) Final color
    let final_color = vec4<f32>(lighting_color.rgb, 1.0);

    // Convert float color to integer
    let R = u32(final_color.r * 255.0);
    let G = u32(final_color.g * 255.0);
    let B = u32(final_color.b * 255.0);
    let A = u32(final_color.a * 255.0);

    let output_color = rgba(R, G, B, A);
    output_buffer[idx] = output_color;

    depth_buffer[idx] = 0xFFFFFFFFu;
}