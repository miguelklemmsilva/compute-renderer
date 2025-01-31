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

struct MaterialBuffer {
    infos: array<Material>,
};

struct TextureInfo {
    offset: u32,
    width: u32,
    height: u32,
    _padding: u32,
};

struct Material {
    texture_info: TextureInfo,
    ambient: vec3<f32>,
    _padding1: f32,
    specular: vec3<f32>,
    _padding2: f32,
    diffuse: vec3<f32>,
    shininess: f32,
    dissolve: f32,
    optical_density: f32,
}

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
    depth: atomic<u32>,
    uv: vec2<f32>,
    normal: vec3<f32>,
    world_pos: vec3<f32>,
    material_index: u32,
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
@group(4) @binding(1) var<storage, read> material_buffer: MaterialBuffer;

@group(5) @binding(0) var<uniform> effect: EffectUniform;

// The fragment data & count from the raster pass
@group(6) @binding(0) var<storage, read> fragment_buffer: FragmentBuffer;

fn rgba(r: u32, g: u32, b: u32, a: u32) -> u32 {
    // BGRA format (0xFF for alpha)
    return (a << 24) | (b << 16) | (g << 8) | r;
}

fn calculate_diffuse_lighting(normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return max(dot(normalize(normal), normalize(light_dir)), 0.0);
}

fn sample_texture(uv: vec2<f32>, texture_info: TextureInfo) -> vec3<f32> {
    if texture_info.offset > arrayLength(&texture_buffer.data) {
        return vec3<f32>(1.0, 1.0, 1.0);
    }

    let tex_width = f32(texture_info.width);
    let tex_height = f32(texture_info.height);

    let u = clamp(uv.x, 0.0, 1.0);
    let v = clamp(uv.y, 0.0, 1.0);

    let x = u32(u * (tex_width - 1.0));
    let y = u32(v * (tex_height - 1.0));
    let index = texture_info.offset + y * texture_info.width + x;
    let texel = texture_buffer.data[index];

    let r = f32((texel >> 24) & 0xFFu) / 255.0;
    let g = f32((texel >> 16) & 0xFFu) / 255.0;
    let b = f32((texel >> 8) & 0xFFu) / 255.0;
    return vec3<f32>(r, g, b);
}

fn calculate_lighting(normal: vec3<f32>, position: vec3<f32>, material: Material, uv: vec2<f32>) -> vec3<f32> {
    // Basic lighting calculation
    var final_color = vec3<f32>(0.0);
    let ambient = vec3<f32>(0.1);
    
    // Start with ambient light
    final_color = ambient;
    
    // Add contribution from each light
    for (var i = 0u; i < 8u; i++) {
        let light = light_buffer.lights[i];
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

    // Early-out if there's no valid fragment
    if idx >= arrayLength(&fragment_buffer.frags) || fragment_buffer.frags[idx].depth == 0xFFFFFFFFu {
        return;
    }

    var normal = fragment_buffer.frags[idx].normal;

    // 2) Retrieve the material
    let mat_index = fragment_buffer.frags[idx].material_index;
    let material = material_buffer.infos[mat_index];

    // 3) Calculate lighting with the material
    let lighting_color = calculate_lighting(normal, fragment_buffer.frags[idx].world_pos, material, fragment_buffer.frags[idx].uv);

    // 4) Handle transparency
    let alpha = material.dissolve;

    // 6) Final color
    let final_color = vec4<f32>(lighting_color.rgb, alpha);

    // Convert float color to integer
    let R = u32(final_color.r * 255.0);
    let G = u32(final_color.g * 255.0);
    let B = u32(final_color.b * 255.0);
    let A = u32(final_color.a * 255.0);

    let output_color = rgba(R, G, B, A);
    output_buffer.data[idx] = output_color;
}