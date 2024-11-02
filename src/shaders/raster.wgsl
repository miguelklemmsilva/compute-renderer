struct OutputBuffer {
    data: array<u32>,
};

struct DepthBuffer {
    depth: array<f32>,
};

struct Vertex { x: f32, y: f32, z: f32 }

struct VertexBuffer {
  values: array<Vertex>,
}

struct Uniform {
  width: f32,
  height: f32,
}

@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;
@group(1) @binding(0) var<storage, read_write> depth_buffer: DepthBuffer;
@group(2) @binding(0) var<uniform> screen_dims : Uniform;
@group(3) @binding(0) var<storage, read> vertex_buffer : VertexBuffer;

fn project(v: Vertex) -> vec3<f32> {
    // convert the vertex to screen space
    return vec3<f32>(
        ((v.x + 1.0) * 0.5) * screen_dims.width,
        (1.0 - (v.y + 1.0) * 0.5) * screen_dims.height,
        v.z
    );
}

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
}

fn color_pixel(x: f32, y: f32, color: u32) {
    let pixelID = u32(x) + u32(y) * u32(screen_dims.width);

    output_buffer.data[pixelID] = color;
}

fn draw_line(v1: vec3<f32>, v2: vec3<f32>) {
  let v1Vec = vec2<f32>(v1.x, v1.y);
  let v2Vec = vec2<f32>(v2.x, v2.y);

  let dist = i32(distance(v1Vec, v2Vec));
  for (var i = 0; i < dist; i = i + 1) {
    let x = v1.x + f32(v2.x - v1.x) * (f32(i) / f32(dist));
    let y = v1.y + f32(v2.y - v1.y) * (f32(i) / f32(dist));
    color_pixel(x, y, rgb(255u, 255u, 255u));
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

    draw_line(v1, v2);
    draw_line(v2, v3);
    draw_line(v3, v1);
}