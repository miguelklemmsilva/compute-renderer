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

@group(0) @binding(0) var<storage, read_write> pixel_color_buffer: OutputBuffer;
@group(1) @binding(0) var<storage, read_write> pixel_depth_buffer: DepthBuffer;
@group(2) @binding(0) var<uniform> screen_dims : Uniform;
@group(3) @binding(0) var<storage, read> vertex_buffer : VertexBuffer;

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
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

    // Set color to a default value (e.g., black)
    pixel_color_buffer.data[idx] = rgb(255u, 255u, 255u);

    // Set depth to maximum (1.0)
    pixel_depth_buffer.depth[idx] = 1.0;
}

@compute @workgroup_size(256, 1)
fn raster(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let width = u32(screen_dims.width);
    let height = u32(screen_dims.height);
    let total_pixels = width * height;

    if (idx >= total_pixels) {
        return;
    }

    // Set color to a default value (e.g., black)
    pixel_color_buffer.data[idx] = rgb(255u, 255u, 255u);

    // Set depth to maximum (1.0)
    pixel_depth_buffer.depth[idx] = 1.0;
}