struct OutputBuffer {
    data: array<u32>,
};

struct DepthBuffer {
    depth: array<f32>,
};

struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    nx: f32,
    ny: f32,
    nz: f32,
    u: f32,
    v: f32,
    texture_index: u32,
};

struct VertexBuffer {
    values: array<Vertex>
};

struct Uniform {
  width: f32,
  height: f32,
}

struct Camera {
  view_pos: vec4<f32>,
  view_proj: mat4x4<f32>,
}

struct TextureBuffer {
    data: array<u32>
};

struct TextureInfos {
    infos: array<TextureInfo>
};

struct TextureInfo {
    offset: u32,
    width: u32,
    height: u32,
    _padding: u32, // Match the Rust struct definition
};

@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;
@group(1) @binding(0) var<storage, read_write> depth_buffer: DepthBuffer;
@group(2) @binding(0) var<uniform> screen_dims : Uniform;
@group(3) @binding(0) var<storage, read> vertex_buffer : VertexBuffer;
@group(4) @binding(0) var<uniform> camera : Camera;
@group(5) @binding(0) var<storage, read> texture_buffer: TextureBuffer;
@group(6) @binding(0) var<storage, read> texture_infos: TextureInfos;

fn calculate_diffuse_lighting(normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return max(dot(normalize(normal), normalize(light_dir)), 0.0);
}

fn sample_texture(uv: vec2<f32>, texture_index: u32) -> vec4<f32> {
  let NO_TEXTURE_INDEX: u32 = 0xffffffffu;

    if (texture_index == NO_TEXTURE_INDEX) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
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

fn project(v: Vertex) -> Vertex {
    // Transform the vertex position to clip space
    let clip_pos = camera.view_proj * vec4<f32>(v.x, v.y, v.z, 1.0);

    // Perform the perspective divide to get NDC
    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert NDC to screen coordinates
    let screen_pos = vec3<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        ndc_pos.z
    );

    // Pass through normals
    return Vertex(
        screen_pos.x,
        screen_pos.y,
        screen_pos.z,
        v.nx,
        v.ny,
        v.nz,
        v.u,
        v.v,
        v.texture_index
    );
}

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
}

fn color_pixel(x: u32, y: u32, depth: f32, color: u32) {
    let pixelID = x + y * u32(screen_dims.width);

    // Check the depth buffer
    if (depth < depth_buffer.depth[pixelID]) {
        // Update depth and color if the new depth is closer
        depth_buffer.depth[pixelID] = depth;
        output_buffer.data[pixelID] = color;
    }
}

fn barycentric(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
  let u = cross(
    vec3<f32>(v3.x - v1.x, v2.x - v1.x, v1.x - p.x), 
    vec3<f32>(v3.y - v1.y, v2.y - v1.y, v1.y - p.y)
  );

  if (abs(u.z) < 1.0) {
    return vec3<f32>(-1.0, 1.0, 1.0);
  }

  return vec3<f32>(1.0 - (u.x+u.y)/u.z, u.y/u.z, u.x/u.z); 
}

fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
  var min_max = vec4<f32>();
  min_max.x = min(min(v1.x, v2.x), v3.x);
  min_max.y = min(min(v1.y, v2.y), v3.y);
  min_max.z = max(max(v1.x, v2.x), v3.x);
  min_max.w = max(max(v1.y, v2.y), v3.y);

  return min_max;
}

fn draw_triangle(v1: Vertex, v2: Vertex, v3: Vertex) {
    let e1 = vec2<f32>(v2.x - v1.x, v2.y - v1.y);
    let e2 = vec2<f32>(v3.x - v1.x, v3.y - v1.y);
    let cross_z = e1.x * e2.y - e1.y * e2.x;

    if (cross_z <= 0.0) {
        // Triangle is back-facing, cull it
        return;
    }

  let texture_index = v1.texture_index;
  let min_max = get_min_max(vec3<f32>(v1.x, v1.y, v1.z), vec3<f32>(v2.x, v2.y, v2.z), vec3<f32>(v3.x, v3.y, v3.z));
  let startX = u32(clamp(min_max.x, 0.0, f32(screen_dims.width - 1)));
  let startY = u32(clamp(min_max.y, 0.0, f32(screen_dims.height - 1)));
  let endX = u32(clamp(min_max.z, 0.0, f32(screen_dims.width - 1)));
  let endY = u32(clamp(min_max.w, 0.0, f32(screen_dims.height - 1)));

    for (var x: u32 = startX; x <= endX; x = x + 1u) {
        for (var y: u32 = startY; y <= endY; y = y + 1u) {
            let bc = barycentric(
                vec3<f32>(v1.x, v1.y, v1.z),
                vec3<f32>(v2.x, v2.y, v2.z),
                vec3<f32>(v3.x, v3.y, v3.z),
                vec2<f32>(f32(x), f32(y))
            );

            if (bc.x < 0.0 || bc.y < 0.0 || bc.z < 0.0) {
                continue;
            }

            let z_value = bc.x * v1.z + bc.y * v2.z + bc.z * v3.z;
            let normalized_z = (z_value + 1.0) * 0.5;

            // Interpolate UV coordinates
            let uv = bc.x * vec2<f32>(v1.u, v1.v) +
                    bc.y * vec2<f32>(v2.u, v2.v) +
                    bc.z * vec2<f32>(v3.u, v3.v);

            // Interpolate normals
            let normal = normalize(
                bc.x * vec3<f32>(v1.nx, v1.ny, v1.nz) +
                bc.y * vec3<f32>(v2.nx, v2.ny, v2.nz) +
                bc.z * vec3<f32>(v3.nx, v3.ny, v3.nz)
            );

            // Compute diffuse lighting
            let light_dir = normalize(vec3<f32>(0.0, 0.0, -1.0)); // Light coming from the camera direction
            let diffuse = calculate_diffuse_lighting(normal, light_dir);

            // Sample the texture
            let tex_color = sample_texture(uv, texture_index);

            // Apply diffuse lighting
            let final_color = tex_color.rgb * diffuse;

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

    if (idx >= total_pixels) {
        return;
    }

    // Set color to a default value
    output_buffer.data[idx] = rgb(0u, 0u, 0u);

    // Set depth to maximum (1.0)
    depth_buffer.depth[idx] = 1.0;
}

@compute @workgroup_size(256, 1)
fn raster(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_idx = global_id.x * 3u;

    let v1 = project(vertex_buffer.values[triangle_idx]);
    let v2 = project(vertex_buffer.values[triangle_idx + 1u]);
    let v3 = project(vertex_buffer.values[triangle_idx + 2u]);

    draw_triangle(v1, v2, v3);
}
