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

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
}

fn color_vertex(vertex: Vertex, color: u32) {
    let x_screen = ((vertex.x + 1.0) * 0.5) * screen_dims.width;
    let y_screen = ((1.0 - (vertex.y + 1.0) * 0.5) * screen_dims.height);

    // Calculate the pixel ID based on the screen space coordinates
    let pixelID = u32(x_screen) + u32(y_screen) * u32(screen_dims.width);

        output_buffer.data[pixelID] = color;
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

    let vertex0 = vertex_buffer.values[triangle_idx];
    let vertex1 = vertex_buffer.values[triangle_idx + 1u];
    let vertex2 = vertex_buffer.values[triangle_idx + 2u];

    color_vertex(vertex0, rgb(255u, 0u, 0u));
    color_vertex(vertex1, rgb(0u, 255u, 0u));
    color_vertex(vertex2, rgb(0u, 0u, 255u));
}