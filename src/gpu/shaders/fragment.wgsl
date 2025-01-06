// -----------------------------------------------------------------------------
// FRAGMENT STAGE
// -----------------------------------------------------------------------------
// We take each emitted Fragment, perform the final shading (lighting + texturing),
// depth test, and write to the screen output.

struct OutputBuffer {
    data: array<atomic<u32>>,
};

struct Uniform {
    width: f32,
    height: f32,
};

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct TextureBuffer {
    data: array<u32>,
};

struct TextureInfos {
    infos: array<TextureInfo>,
};

struct TextureInfo {
    offset: u32,
    width: u32,
    height: u32,
    _padding: u32,
};

struct Light {
    world_position: vec3<f32>,
    _padding1: f32,
    view_position: vec3<f32>,
    _padding2: f32,
    color: vec3<f32>,
    intensity: f32,
};

struct LightBuffer {
    lights: array<Light>,
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
    screen_x: u32,
    screen_y: u32,
    depth: atomic<u32>,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    texture_index: u32,
};

struct FragmentBuffer {
    frags: array<Fragment>,
};

// -----------------------------------------------------------------------------
// BINDINGS
// -----------------------------------------------------------------------------
@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read> light_buffer: LightBuffer;

@group(4) @binding(0) var<storage, read> texture_buffer: TextureBuffer;
@group(4) @binding(1) var<storage, read> texture_infos: TextureInfos;

@group(5) @binding(0) var<uniform> effect: EffectUniform;

// The fragment data & count from the raster pass
@group(6) @binding(0) var<storage, read> fragment_buffer: FragmentBuffer;

// -----------------------------------------------------------------------------
// HELPER FUNCTIONS (lighting, texturing, depth test, etc.)
// -----------------------------------------------------------------------------
fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
}

fn calculate_diffuse_lighting(normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return max(dot(normalize(normal), normalize(light_dir)), 0.0);
}

fn sample_texture(uv: vec2<f32>, texture_index: u32) -> vec4<f32> {
    let NO_TEXTURE_INDEX: u32 = 0xffffffffu;
    if texture_index == NO_TEXTURE_INDEX {
        return vec4<f32>(0.8, 0.8, 0.8, 1.0);
    }

    let tex_info = texture_infos.infos[texture_index];
    let tex_width = f32(tex_info.width);
    let tex_height = f32(tex_info.height);

    let u = clamp(uv.x, 0.0, 1.0);
    let v = clamp(1.0 - uv.y, 0.0, 1.0);

    let x = u32(u * (tex_width - 1.0));
    let y = u32(v * (tex_height - 1.0));
    let index = tex_info.offset + y * tex_info.width + x;
    let texel = texture_buffer.data[index];

    let r = f32((texel >> 24) & 0xFFu) / 255.0;
    let g = f32((texel >> 16) & 0xFFu) / 255.0;
    let b = f32((texel >> 8)  & 0xFFu) / 255.0;
    let a = f32(texel & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

fn apply_dissolve_effect(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    // You'd need hash/noise again if you do the dissolve effect. Omitted for brevity,
    // but you can copy from your original code. We'll just return color here
    // if you want to show the same logic, replicate it exactly.
    return color; // placeholder
}

fn apply_smooth_to_flat_effect(normal: vec3<f32>, effect: EffectUniform) -> vec3<f32> {
    let progress = effect.param1;
    let up = vec3<f32>(0.0, 1.0, 0.0);
    return normalize(mix(normal, up, progress));
}

fn calculate_lighting(normal: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let normal_normalized = normalize(normal);
    let ambient = vec3<f32>(0.1, 0.1, 0.1);
    var total_diffuse = vec3<f32>(0.0);
    var total_specular = vec3<f32>(0.0);

    for (var i = 0u; i < arrayLength(&light_buffer.lights); i = i + 1u) {
        let light = light_buffer.lights[i];
        let light_dir = normalize(light.world_position - position);
        let light_distance = length(light.world_position - position);
        let attenuation = 1.0 / (1.0 + 0.1 * light_distance + 0.01 * light_distance * light_distance);

        let diff = max(dot(normal_normalized, light_dir), 0.0);
        let diffuse = light.color * diff * light.intensity * attenuation;
        total_diffuse = total_diffuse + diffuse;

        let view_dir = normalize(camera.view_pos.xyz - position);
        let reflect_dir = reflect(-light_dir, normal_normalized);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        let specular = light.color * spec * light.intensity * attenuation * 0.5;
        total_specular = total_specular + specular;
    }

    return min(ambient + total_diffuse + total_specular, vec3<f32>(1.0));
}

// -----------------------------------------------------------------------------
// ENTRY POINT
// -----------------------------------------------------------------------------
@compute @workgroup_size(256)
fn fragment_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x + global_id.y * u32(screen_dims.width);

    if idx >= arrayLength(&fragment_buffer.frags) || fragment_buffer.frags[idx].depth == 0xFFFFFFFFu {
        return;
    }

    // Possibly apply normal effect
    var normal = fragment_buffer.frags[idx].normal;
    if effect.effect_type == 3u { // SmoothToFlat
        normal = apply_smooth_to_flat_effect(normal, effect);
    }

    // Sample texture
    let tex_color = sample_texture(fragment_buffer.frags[idx].uv, fragment_buffer.frags[idx].texture_index);

    // Compute lighting
    let lighting = calculate_lighting(normal, fragment_buffer.frags[idx].world_pos);
    var final_color = vec4<f32>(tex_color.rgb * lighting, tex_color.a);

    // Possibly apply dissolve
    if effect.effect_type == 2u {
        final_color = apply_dissolve_effect(final_color, fragment_buffer.frags[idx].uv);
    }

    // Convert float color to integer
    let R = u32(final_color.r * 255.0);
    let G = u32(final_color.g * 255.0);
    let B = u32(final_color.b * 255.0);

    let color = rgb(R, G, B);

    atomicStore(&output_buffer.data[idx], color);
}