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

fn color_pixel(x: u32, y: u32, color: u32) {
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
    color_pixel(u32(x), u32(y), rgb(255u, 255u, 255u));
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

fn draw_triangle(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) {
  let min_max = get_min_max(v1, v2, v3);
  let startX = u32(min_max.x);
  let startY = u32(min_max.y);
  let endX = u32(min_max.z);
  let endY = u32(min_max.w);

  for (var x: u32 = startX; x <= endX; x = x + 1u) {
    for (var y: u32 = startY; y <= endY; y = y + 1u) {
      let bc = barycentric(v1, v2, v3, vec2<f32>(f32(x), f32(y))); 

      if (bc.x < 0.0 || bc.y < 0.0 || bc.z < 0.0) {
        continue;
      }

      // Normalize z-value from [-1.0, 1.0] to [0.0, 1.0]
      let z_value = bc.x * v1.z + bc.y * v2.z + bc.z * v3.z;
      let normalized_z = (z_value + 1.0) * 0.5;

      // Scale to [0, 255] and convert to u32
      let color = u32(normalized_z * 255.0);

      let R = u32(normalized_z * 255.0); // Red varies with depth
      let G = u32((1.0 - normalized_z) * 255.0); // Green inversely varies with depth
      let B = u32(bc.z * 255.0); // Blue based on third barycentric weight

      color_pixel(x, y, rgb(R, G, B));
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
    output_buffer.data[idx] = rgb(255u, 255u, 255u);

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