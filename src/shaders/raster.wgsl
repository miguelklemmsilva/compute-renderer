struct OutputBuffer {
    data: array<atomic<u32>>,
};

struct DepthBuffer {
    depth: array<atomic<u32>>,
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
    values: array<Vertex>
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
}

@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;
@group(0) @binding(1) var<storage, read_write> depth_buffer: DepthBuffer;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read> vertex_buffer: VertexBuffer;
@group(3) @binding(1) var<storage, read> lights: LightBuffer;

@group(4) @binding(0) var<storage, read> texture_buffer: TextureBuffer;
@group(4) @binding(1) var<storage, read> texture_infos: TextureInfos;

@group(5) @binding(0) var<uniform> effect: EffectUniform;

fn calculate_diffuse_lighting(normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return max(dot(normalize(normal), normalize(light_dir)), 0.0);
}

fn sample_texture(uv: vec2<f32>, texture_index: u32) -> vec4<f32> {
    let NO_TEXTURE_INDEX: u32 = 0xffffffffu;

    if texture_index == NO_TEXTURE_INDEX {
        return vec4<f32>(0.8, 0.8, 0.8, 1.0); // Neutral gray for no texture
    }

    let tex_info = texture_infos.infos[texture_index];
    let tex_width = f32(tex_info.width);
    let tex_height = f32(tex_info.height);

    let u = clamp(uv.x, 0.0, 1.0);
    let v = clamp(1.0 - uv.y, 0.0, 1.0);

    let x = u32(u * (tex_width - 1.0));
    let y = u32(v * (tex_height - 1.0));

    let tex_index = tex_info.offset + y * tex_info.width + x;
    let texel = texture_buffer.data[tex_index];

    let r = f32((texel >> 24) & 0xFFu) / 255.0;
    let g = f32((texel >> 16) & 0xFFu) / 255.0;
    let b = f32((texel >> 8) & 0xFFu) / 255.0;
    let a = f32(texel & 0xFFu) / 255.0;

    return vec4<f32>(r, g, b, a);
}

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

fn apply_wave_effect(pos: vec3<f32>) -> vec3<f32> {
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

fn apply_dissolve_effect(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let threshold = effect.param1;
    let noise_scale = effect.param2;
    let n = noise(uv * noise_scale);
    
    if n < threshold {
        return vec4<f32>(0.0);
    }
    return color;
}

fn apply_smooth_to_flat_effect(normal: vec3<f32>) -> vec3<f32> {
    let progress = effect.param1;
    let up = vec3<f32>(0.0, 1.0, 0.0);
    return normalize(mix(normal, up, progress));
}

fn apply_pixelate_effect(uv: vec2<f32>) -> vec2<f32> {
    let pixel_size = effect.param1;
    return floor(uv * pixel_size) / pixel_size;
}

fn apply_voxelize_effect(pos: vec3<f32>) -> vec3<f32> {
    let grid_size = effect.param1;
    return floor(pos * grid_size) / grid_size;
}

fn project(v: Vertex) -> Vertex {
    var modified_v = v;
    var world_pos = vec3<f32>(v.x, v.y, v.z);
    
    // Apply position-based effects before projection
    if effect.effect_type == 1u { // Wave
        world_pos = apply_wave_effect(world_pos);
    } else if effect.effect_type == 5u { // Voxelize
        world_pos = apply_voxelize_effect(world_pos);
    }

    // Transform the vertex position to clip space
    let clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // Perform the perspective divide to get NDC
    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert NDC to screen coordinates
    let screen_pos = vec3<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        ndc_pos.z
    );

    // Return the modified Vertex with original world position stored
    return Vertex(
        screen_pos.x,
        screen_pos.y,
        screen_pos.z,
        modified_v.u,
        modified_v.v,
        world_pos.x,  // Store world position in normal fields
        world_pos.y,  // for use in lighting calculations
        world_pos.z,
        modified_v.texture_index,
        clip_pos.w
    );
}

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
}

fn float_to_depth_int(depth: f32) -> u32 {
    return u32(depth * 4294967295.0);
}

fn color_pixel(x: u32, y: u32, depth: f32, color: u32) {
    let pixelID = x + y * u32(screen_dims.width);
    let depth_int = float_to_depth_int(depth);

    loop {
        let old_depth = atomicLoad(&depth_buffer.depth[pixelID]);
        // Attempt to update the depth buffer only if the new depth is closer
        if depth_int < old_depth {
            let exchanged = atomicCompareExchangeWeak(&depth_buffer.depth[pixelID], old_depth, depth_int);
            if exchanged.exchanged {
                // Successfully updated depth buffer; update color buffer
                atomicExchange(&output_buffer.data[pixelID], color);
                break;
            }
        } else {
            break; // Depth is not closer; exit
        }
    }
}

fn barycentric(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    let u = cross(
        vec3<f32>(v3.x - v1.x, v2.x - v1.x, v1.x - p.x),
        vec3<f32>(v3.y - v1.y, v2.y - v1.y, v1.y - p.y)
    );

    if abs(u.z) < 1.0 {
        return vec3<f32>(-1.0, 1.0, 1.0);
    }

    return vec3<f32>(1.0 - (u.x + u.y) / u.z, u.y / u.z, u.x / u.z);
}

fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    var min_max = vec4<f32>();
    min_max.x = min(min(v1.x, v2.x), v3.x);
    min_max.y = min(min(v1.y, v2.y), v3.y);
    min_max.z = max(max(v1.x, v2.x), v3.x);
    min_max.w = max(max(v1.y, v2.y), v3.y);

    return min_max;
}

fn calculate_lighting(normal: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let normal_normalized = normalize(normal);
    let ambient = vec3<f32>(0.1, 0.1, 0.1);
    var total_diffuse = vec3<f32>(0.0);
    var total_specular = vec3<f32>(0.0);

    for (var i = 0u; i < arrayLength(&lights.lights); i = i + 1u) {
        let light = lights.lights[i];
        
        // Calculate light direction in world space
        let light_dir = normalize(light.world_position - position);
        let light_distance = length(light.world_position - position);
        let attenuation = 1.0 / (1.0 + 0.1 * light_distance + 0.01 * light_distance * light_distance);

        // Diffuse
        let diff = max(dot(normal_normalized, light_dir), 0.0);
        let diffuse = light.color * diff * light.intensity * attenuation;
        total_diffuse = total_diffuse + diffuse;

        // Specular
        let view_dir = normalize(camera.view_pos.xyz - position);
        let reflect_dir = reflect(-light_dir, normal_normalized);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        let specular = light.color * spec * light.intensity * attenuation * 0.5;
        total_specular = total_specular + specular;
    }

    // Combine ambient, diffuse, and specular, ensuring we don't exceed 1.0
    return min(ambient + total_diffuse + total_specular, vec3<f32>(1.0));
}

fn draw_triangle(v1: Vertex, v2: Vertex, v3: Vertex) {
    let texture_index = v1.texture_index;

    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

    let startX = u32(clamp(min_max.x, 0.0, screen_dims.width - 1.0));
    let startY = u32(clamp(min_max.y, 0.0, screen_dims.height - 1.0));
    let endX = u32(clamp(min_max.z, 0.0, screen_dims.width - 1.0));
    let endY = u32(clamp(min_max.w, 0.0, screen_dims.height - 1.0));

    // Get world positions from the normal fields where we stored them
    let world_pos1 = vec3<f32>(v1.nx, v1.ny, v1.nz);
    let world_pos2 = vec3<f32>(v2.nx, v2.ny, v2.nz);
    let world_pos3 = vec3<f32>(v3.nx, v3.ny, v3.nz);

    for (var x: u32 = startX; x <= endX; x = x + 1u) {
        for (var y: u32 = startY; y <= endY; y = y + 1u) {
            let bc = barycentric(
                vec3<f32>(v1.x, v1.y, v1.z),
                vec3<f32>(v2.x, v2.y, v2.z),
                vec3<f32>(v3.x, v3.y, v3.z),
                vec2<f32>(f32(x), f32(y))
            );

            if bc.x < 0.0 || bc.y < 0.0 || bc.z < 0.0 {
                continue;
            }

            // Compute reciprocal of w_clip for each vertex
            let one_over_w1 = 1.0 / v1.w_clip;
            let one_over_w2 = 1.0 / v2.w_clip;
            let one_over_w3 = 1.0 / v3.w_clip;

            // Interpolated 1/w
            let interpolated_one_over_w = bc.x * one_over_w1 + bc.y * one_over_w2 + bc.z * one_over_w3;

            // Interpolate world position
            let world_pos = (world_pos1 * bc.x + world_pos2 * bc.y + world_pos3 * bc.z);

            // Interpolate UV coordinates
            let interpolated_uv_over_w = bc.x * vec2<f32>(v1.u, v1.v) * one_over_w1 +
                                       bc.y * vec2<f32>(v2.u, v2.v) * one_over_w2 +
                                       bc.z * vec2<f32>(v3.u, v3.v) * one_over_w3;
            var uv = interpolated_uv_over_w / interpolated_one_over_w;

            // Apply UV-based effects
            if effect.effect_type == 4u { // Pixelate
                uv = apply_pixelate_effect(uv);
            }

            // Interpolate normal (perspective-correct)
            let normal_over_w = (bc.x * vec3<f32>(v1.nx, v1.ny, v1.nz) * one_over_w1 +
                                bc.y * vec3<f32>(v2.nx, v2.ny, v2.nz) * one_over_w2 +
                                bc.z * vec3<f32>(v3.nx, v3.ny, v3.nz) * one_over_w3);
            var interpolated_normal = normalize(normal_over_w / interpolated_one_over_w);

            // Apply normal-based effects
            if effect.effect_type == 3u { // Smooth to flat
                interpolated_normal = apply_smooth_to_flat_effect(interpolated_normal);
            }

            // Interpolate depth
            let interpolated_z_over_w = bc.x * v1.z * one_over_w1 +
                                      bc.y * v2.z * one_over_w2 +
                                      bc.z * v3.z * one_over_w3;
            let interpolated_z = interpolated_z_over_w / interpolated_one_over_w;
            let normalized_z = clamp(interpolated_z, 0.0, 1.0);

            // Sample the texture
            let tex_color = sample_texture(uv, texture_index);

            // Calculate lighting using world-space position and normal
            let lighting = calculate_lighting(interpolated_normal, world_pos);

            // Apply lighting to texture color
            var final_color = vec4<f32>(tex_color.rgb * lighting, tex_color.a);

            // Apply color-based effects
            if effect.effect_type == 2u { // Dissolve
                final_color = apply_dissolve_effect(final_color, uv);
            }

            // Convert color to u32
            let R = u32(final_color.r * 255.0);
            let G = u32(final_color.g * 255.0);
            let B = u32(final_color.b * 255.0);

            color_pixel(x, y, normalized_z, rgb(R, G, B));
        }
    }
}

@compute @workgroup_size(256, 1)
fn clear(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let width = u32(screen_dims.width);
    let height = u32(screen_dims.height);
    let total_pixels = width * height;

    if idx >= total_pixels {
        return;
    }

    // Set color to a default value
    output_buffer.data[idx] = rgb(0u, 0u, 0u);

    // Set depth to maximum value (farthest depth)
    atomicStore(&depth_buffer.depth[idx], 0xFFFFFFFFu);
}

@compute @workgroup_size(256, 1)
fn raster(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_idx = global_id.x * 3u;

    // Ensure we don't access out of bounds
    if triangle_idx + 2u >= arrayLength(&vertex_buffer.values) {
        return;
    }

    let v1 = project(vertex_buffer.values[triangle_idx]);
    let v2 = project(vertex_buffer.values[triangle_idx + 1u]);
    let v3 = project(vertex_buffer.values[triangle_idx + 2u]);

    // Back Face Culling
    // let edge1 = vec2<f32>(v2.x - v1.x, v2.y - v1.y);
    // let edge2 = vec2<f32>(v3.x - v1.x, v3.y - v1.y);
    // let face_normal_z = edge1.x * edge2.y - edge1.y * edge2.x;

    // if face_normal_z <= 0.0 {
    //     // The triangle is back-facing; cull it
    //     return;
    // }

    draw_triangle(v1, v2, v3);
}