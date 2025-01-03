// This structure is made writable so that multiple threads can update the final
// on-screen color buffer concurrently. Using an atomic array allows for safe parallel
// writes, especially in combination with depth testing.
struct OutputBuffer {
    data: array<atomic<u32>>,
};

// A dedicated structure for the depth buffer is used so we can run depth testing
// independently from the color buffer. This is crucial for correct ordering of drawn
// fragments in 3D.
struct DepthBuffer {
    depth: array<atomic<u32>>,
};

// Vertices hold position, texture coordinates, normal vectors, texture indices,
// and a w_clip value for perspective-correct interpolation and depth calculations.
// We store additional data in the normal fields (like world position when needed) to
// avoid extra buffers during rasterization and lighting.
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

// A buffer of vertices, designed for reading in a GPU compute pipeline. Storing
// them in an array allows us to process triangles in batches of three.
struct VertexBuffer {
    values: array<Vertex>
};

// Holds screen dimensions. This is needed to map normalized device coordinates
// to actual pixel coordinates and to ensure bounding during rasterization.
struct Uniform {
    width: f32,
    height: f32,
};

// Contains the camera's position and its view-projection matrix. The camera
// position is needed for certain lighting computations (e.g. specular highlights).
struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

// Holds raw texture data for all textures in a single array, which can then be
// sampled with offsets and dimensions specified by TextureInfo entries.
struct TextureBuffer {
    data: array<u32>,
};

// Maintains metadata (offset in the texture buffer, width, height, etc.) for
// each texture. This approach helps handle multiple textures in a single buffer.
struct TextureInfos {
    infos: array<TextureInfo>,
};

// Contains per-texture metadata, letting us know how to index into the shared
// texture buffer and how to handle sampling (dimensions, etc.).
struct TextureInfo {
    offset: u32,
    width: u32,
    height: u32,
    _padding: u32,
};

// Represents a single light in the scene, including both world and view-space
// positions (depending on usage). The color and intensity drive diffuse
// and specular computations.
struct Light {
    world_position: vec3<f32>,
    _padding1: f32,
    view_position: vec3<f32>,
    _padding2: f32,
    color: vec3<f32>,
    intensity: f32,
};

// Stores multiple lights that can be looped over for cumulative lighting effects.
// This approach supports adding many lights in a straightforward manner.
struct LightBuffer {
    lights: array<Light>,
};

// A generic structure to control an assortment of post-processing or vertex-based
// effects. The effect_type determines which effect is active, while the params
// allow fine-grained adjustments (like intensity, frequency, etc.).
struct EffectUniform {
    effect_type: u32,
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
    time: f32,
    _padding: vec2<f32>,
}

// Binding groups specify which data is visible to each shader stage or function.
// This makes sure we can write to the output and depth buffers, read from uniform
// data, etc. in a consistent layout.
@group(0) @binding(0) var<storage, read_write> output_buffer: OutputBuffer;
@group(0) @binding(1) var<storage, read_write> depth_buffer: DepthBuffer;

@group(1) @binding(0) var<uniform> screen_dims: Uniform;
@group(2) @binding(0) var<uniform> camera: Camera;

@group(3) @binding(0) var<storage, read> vertex_buffer: VertexBuffer;
@group(3) @binding(1) var<storage, read> lights: LightBuffer;

@group(4) @binding(0) var<storage, read> texture_buffer: TextureBuffer;
@group(4) @binding(1) var<storage, read> texture_infos: TextureInfos;

@group(5) @binding(0) var<uniform> effect: EffectUniform;

// A helper function to compute diffuse lighting by taking the dot product
// of normalized vectors. We keep it separate to remain modular and clean.
fn calculate_diffuse_lighting(normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    return max(dot(normalize(normal), normalize(light_dir)), 0.0);
}

// Samples from our shared texture buffer, using the corresponding offset
// and per-texture dimensions to find the correct texel. The reason for
// storing in a large array is to avoid multiple bindings for multiple textures.
fn sample_texture(uv: vec2<f32>, texture_index: u32) -> vec4<f32> {
    let NO_TEXTURE_INDEX: u32 = 0xffffffffu;

    // If there's no valid texture index, we produce a default neutral color.
    // This handles cases where an object might not need texturing at all.
    if texture_index == NO_TEXTURE_INDEX {
        return vec4<f32>(0.8, 0.8, 0.8, 1.0);
    }

    let tex_info = texture_infos.infos[texture_index];
    let tex_width = f32(tex_info.width);
    let tex_height = f32(tex_info.height);

    // Clamping UVs ensures we don't read outside the texture when sampling,
    // preventing out-of-bounds errors.
    let u = clamp(uv.x, 0.0, 1.0);
    let v = clamp(1.0 - uv.y, 0.0, 1.0);

    let x = u32(u * (tex_width - 1.0));
    let y = u32(v * (tex_height - 1.0));

    // The offset in the texture buffer is computed using row-major layout.
    // This is how we quickly map a 2D (x, y) to the linear texture array.
    let tex_index = tex_info.offset + y * tex_info.width + x;
    let texel = texture_buffer.data[tex_index];

    // Unpacking the RGBA channels from a 32-bit integer. This approach
    // packs each channel into 8 bits, so we shift and mask to retrieve each.
    let r = f32((texel >> 24) & 0xFFu) / 255.0;
    let g = f32((texel >> 16) & 0xFFu) / 255.0;
    let b = f32((texel >> 8) & 0xFFu) / 255.0;
    let a = f32(texel & 0xFFu) / 255.0;

    return vec4<f32>(r, g, b, a);
}

// Simple pseudo-random function that helps generate a noise value for certain
// effects (like dissolve). It's based on fractal hashing of the input vector.
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// 2D noise function built on top of hash() to produce smooth transitions
// between pseudo-random values. This helps with gradual dissolve transitions.
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

// Used to displace a vertex in various ways (vertical, horizontal, radial)
// based on parameters in the EffectUniform. This function is called prior
// to projection so the wave or displacement is reflected in the final draw.
fn apply_wave_effect(pos: vec3<f32>) -> vec3<f32> {
    var modified_pos = pos;
    let amplitude = effect.param1;
    let frequency = effect.param2;
    let phase = effect.param3;
    let direction = effect.param4;

    // We apply different wave patterns (vertical/horizontal/radial) depending
    // on the user-specified direction. This helps create dynamic transformations.
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

// Creates a "dissolve" effect by using noise to selectively discard pixels
// below a threshold. This simulates an object disintegrating.
fn apply_dissolve_effect(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let threshold = effect.param1;
    let noise_scale = effect.param2;
    let n = noise(uv * noise_scale);
    
    // Any part of the object where noise < threshold is "dissolved", so we
    // return a transparent color. Otherwise, we pass the original color.
    if n < threshold {
        return vec4<f32>(0.0);
    }
    return color;
}

// Temporarily blends the mesh normal with a flat up-vector to gradually
// transform smooth shading into a flat style, typically used for stylized
// transitions or artistic effects.
fn apply_smooth_to_flat_effect(normal: vec3<f32>) -> vec3<f32> {
    let progress = effect.param1;
    let up = vec3<f32>(0.0, 1.0, 0.0);
    return normalize(mix(normal, up, progress));
}

// Snaps UV coordinates to a grid for a pixelated aesthetic, often reminiscent
// of retro or 8-bit styles.
fn apply_pixelate_effect(uv: vec2<f32>) -> vec2<f32> {
    let pixel_size = effect.param1;
    return floor(uv * pixel_size) / pixel_size;
}

// Forces coordinates to snap to discrete positions in 3D space, giving a
// blocky "voxelized" look. This is especially noticeable on surfaces that
// become axis-aligned cubes at varying resolutions.
fn apply_voxelize_effect(pos: vec3<f32>) -> vec3<f32> {
    let grid_size = effect.param1;
    return floor(pos * grid_size) / grid_size;
}

// Transforms a single vertex from world space to screen space.
// This includes any position-based effects (e.g., waves, voxelizing) before
// the perspective projection to ensure the final position is modified accordingly.
fn project(v: Vertex) -> Vertex {
    var modified_v = v;
    var world_pos = vec3<f32>(v.x, v.y, v.z);
    
    // Check if there's any active effect that modifies position, and apply it.
    if effect.effect_type != 0u {
        if effect.effect_type == 1u { // Wave
            world_pos = apply_wave_effect(world_pos);
        } else if effect.effect_type == 5u { // Voxelize
            world_pos = apply_voxelize_effect(world_pos);
        }
    }

    // Multiply by the view-projection matrix to get the clip-space position.
    let clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // Divide by w to transform to NDC (Normalized Device Coordinates).
    let ndc_pos = clip_pos.xyz / clip_pos.w;

    // Convert from NDC to screen coordinates. The reason for this mapping
    // is that NDC is in [-1,1], but we need pixel coordinates in [0, width or height].
    let screen_pos = vec3<f32>(
        ((ndc_pos.x + 1.0) * 0.5) * screen_dims.width,
        ((1.0 - ndc_pos.y) * 0.5) * screen_dims.height,
        ndc_pos.z
    );

    // We store the world position in the 'normal' fields to reuse them in
    // lighting calculations without needing a separate buffer.
    return Vertex(
        screen_pos.x,
        screen_pos.y,
        screen_pos.z,
        modified_v.u,
        modified_v.v,
        world_pos.x,
        world_pos.y,
        world_pos.z,
        modified_v.texture_index,
        clip_pos.w
    );
}

// Combines three channels into a single 32-bit value representing an RGB color
// in the usual 0xRRGGBB format. This is eventually stored in output_buffer.
fn rgb(r: u32, g: u32, b: u32) -> u32 {
    return (r << 16) | (g << 8) | b;
}

// Converts a floating-point depth value in [0, 1] into a 32-bit integer. This
// is done so that we can compare depths atomically when updating the depth buffer.
fn float_to_depth_int(depth: f32) -> u32 {
    return u32(depth * 4294967295.0);
}

// Updates a single pixel's color in the output buffer if and only if the
// provided depth is closer than the currently stored depth. The atomic
// compare-exchange ensures only the closest fragment wins.
fn color_pixel(x: u32, y: u32, depth: f32, color: u32) {
    let pixelID = x + y * u32(screen_dims.width);
    let depth_int = float_to_depth_int(depth);

    loop {
        let old_depth = atomicLoad(&depth_buffer.depth[pixelID]);
        // Only proceed if the new depth is nearer (less than) the existing one.
        if depth_int < old_depth {
            let exchanged = atomicCompareExchangeWeak(&depth_buffer.depth[pixelID], old_depth, depth_int);
            if exchanged.exchanged {
                // We successfully updated the depth, so now we update the color.
                atomicExchange(&output_buffer.data[pixelID], color);
                break;
            }
        } else {
            // The fragment is behind the one already in the buffer; ignore it.
            break;
        }
    }
}

// Computes barycentric coordinates for a point in a triangle. We leverage
// these to interpolate values (e.g. UVs, normals, depth) across the triangle.
fn barycentric(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    let u = cross(
        vec3<f32>(v3.x - v1.x, v2.x - v1.x, v1.x - p.x),
        vec3<f32>(v3.y - v1.y, v2.y - v1.y, v1.y - p.y)
    );

    // If u.z is nearly zero, it indicates a degenerate scenario (e.g., triangle
    // with zero area), so we return invalid barycentrics. We rely on negative
    // checks later to discard such fragments.
    if abs(u.z) < 1.0 {
        return vec3<f32>(-1.0, 1.0, 1.0);
    }

    return vec3<f32>(1.0 - (u.x + u.y) / u.z, u.y / u.z, u.x / u.z);
}

// Finds the bounding box in pixel coordinates for a triangle. This ensures we
// only scan over the relevant region, which can significantly improve performance
// by avoiding unnecessary pixel checks.
fn get_min_max(v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec4<f32> {
    var min_max = vec4<f32>();
    min_max.x = min(min(v1.x, v2.x), v3.x);
    min_max.y = min(min(v1.y, v2.y), v3.y);
    min_max.z = max(max(v1.x, v2.x), v3.x);
    min_max.w = max(max(v1.y, v2.y), v3.y);

    return min_max;
}

// Aggregates ambient, diffuse, and specular lighting for each pixel. This
// approach sums contributions from multiple lights, supporting scenes with
// more complex lighting setups.
fn calculate_lighting(normal: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let normal_normalized = normalize(normal);
    let ambient = vec3<f32>(0.1, 0.1, 0.1);
    var total_diffuse = vec3<f32>(0.0);
    var total_specular = vec3<f32>(0.0);

    // Loop through each light to accumulate contributions. We do a distance-based
    // attenuation to ensure far-away lights are weaker.
    for (var i = 0u; i < arrayLength(&lights.lights); i = i + 1u) {
        let light = lights.lights[i];
        
        let light_dir = normalize(light.world_position - position);
        let light_distance = length(light.world_position - position);
        let attenuation = 1.0 / (1.0 + 0.1 * light_distance + 0.01 * light_distance * light_distance);

        let diff = max(dot(normal_normalized, light_dir), 0.0);
        let diffuse = light.color * diff * light.intensity * attenuation;
        total_diffuse = total_diffuse + diffuse;

        // A straightforward specular approximation using the Blinn-Phong model
        // (reflected light vector and a power for shininess).
        let view_dir = normalize(camera.view_pos.xyz - position);
        let reflect_dir = reflect(-light_dir, normal_normalized);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        let specular = light.color * spec * light.intensity * attenuation * 0.5;
        total_specular = total_specular + specular;
    }

    // The final color is a clamped sum of ambient, diffuse, and specular terms.
    return min(ambient + total_diffuse + total_specular, vec3<f32>(1.0));
}

// Rasterizes a single triangle by finding its bounding box, iterating over the
// pixels within, calculating barycentric coords, and performing depth & color
// interpolation. Each pixel is tested for coverage and updated if closer.
fn draw_triangle(v1: Vertex, v2: Vertex, v3: Vertex) {
    let texture_index = v1.texture_index;

    // We get the bounding rectangle of the projected triangle. This narrows
    // the region of interest during rasterization, minimizing wasted work.
    let min_max = get_min_max(
        vec3<f32>(v1.x, v1.y, v1.z),
        vec3<f32>(v2.x, v2.y, v2.z),
        vec3<f32>(v3.x, v3.y, v3.z)
    );

    let startX = u32(clamp(min_max.x, 0.0, screen_dims.width - 1.0));
    let startY = u32(clamp(min_max.y, 0.0, screen_dims.height - 1.0));
    let endX = u32(clamp(min_max.z, 0.0, screen_dims.width - 1.0));
    let endY = u32(clamp(min_max.w, 0.0, screen_dims.height - 1.0));

    // Retrieve original world positions from where we stored them in normal fields.
    let world_pos1 = vec3<f32>(v1.nx, v1.ny, v1.nz);
    let world_pos2 = vec3<f32>(v2.nx, v2.ny, v2.nz);
    let world_pos3 = vec3<f32>(v3.nx, v3.ny, v3.nz);

    // Scan through each pixel in the bounding box. Barycentric coordinates
    // will tell us if the pixel is inside the triangle.
    for (var x: u32 = startX; x <= endX; x = x + 1u) {
        for (var y: u32 = startY; y <= endY; y = y + 1u) {
            let bc = barycentric(
                vec3<f32>(v1.x, v1.y, v1.z),
                vec3<f32>(v2.x, v2.y, v2.z),
                vec3<f32>(v3.x, v3.y, v3.z),
                vec2<f32>(f32(x), f32(y))
            );

            // If any barycentric component is negative, we're outside the triangle.
            if bc.x < 0.0 || bc.y < 0.0 || bc.z < 0.0 {
                continue;
            }

            // We use 1/w for perspective-correct interpolation of depth, UV, etc.
            let one_over_w1 = 1.0 / v1.w_clip;
            let one_over_w2 = 1.0 / v2.w_clip;
            let one_over_w3 = 1.0 / v3.w_clip;

            let interpolated_one_over_w = bc.x * one_over_w1 + bc.y * one_over_w2 + bc.z * one_over_w3;

            let world_pos = (world_pos1 * bc.x + world_pos2 * bc.y + world_pos3 * bc.z);

            // UV interpolation, including perspective correction, ensures textures
            // don't appear warped incorrectly.
            let interpolated_uv_over_w = bc.x * vec2<f32>(v1.u, v1.v) * one_over_w1 +
                                         bc.y * vec2<f32>(v2.u, v2.v) * one_over_w2 +
                                         bc.z * vec2<f32>(v3.u, v3.v) * one_over_w3;
            var uv = interpolated_uv_over_w / interpolated_one_over_w;

            // If a pixelation effect is active, we re-map UV to a coarser grid.
            if effect.effect_type == 4u { // Pixelate
                uv = apply_pixelate_effect(uv);
            }

            // Normal interpolation uses the same perspective-correct approach.
            // We re-use the fields we stored as 'nx, ny, nz' for positions/normal.
            let normal_over_w = (bc.x * vec3<f32>(v1.nx, v1.ny, v1.nz) * one_over_w1 +
                                 bc.y * vec3<f32>(v2.nx, v2.ny, v2.nz) * one_over_w2 +
                                 bc.z * vec3<f32>(v3.nx, v3.ny, v3.nz) * one_over_w3);
            var interpolated_normal = normalize(normal_over_w / interpolated_one_over_w);

            // If we're transitioning from smooth to flat shading, we transform
            // the interpolated normal accordingly.
            if effect.effect_type == 3u { // Smooth to flat
                interpolated_normal = apply_smooth_to_flat_effect(interpolated_normal);
            }

            // Depth is similarly perspective-correct, ensuring objects appear
            // correctly sorted in 3D.
            let interpolated_z_over_w = bc.x * v1.z * one_over_w1 +
                                        bc.y * v2.z * one_over_w2 +
                                        bc.z * v3.z * one_over_w3;
            let interpolated_z = interpolated_z_over_w / interpolated_one_over_w;
            let normalized_z = clamp(interpolated_z, 0.0, 1.0);

            // Fetch the base color from the texture.
            let tex_color = sample_texture(uv, texture_index);

            // Combine the sampled color with dynamic lighting for realistic shading.
            let lighting = calculate_lighting(interpolated_normal, world_pos);
            var final_color = vec4<f32>(tex_color.rgb * lighting, tex_color.a);

            // If there's a dissolve effect, we use noise to discard some pixels.
            if effect.effect_type == 2u { // Dissolve
                final_color = apply_dissolve_effect(final_color, uv);
            }

            // Convert the final float RGBA back to a packed 32-bit value to store in our output.
            let R = u32(final_color.r * 255.0);
            let G = u32(final_color.g * 255.0);
            let B = u32(final_color.b * 255.0);

            // We then attempt to set the pixel color if our depth test passes.
            color_pixel(x, y, normalized_z, rgb(R, G, B));
        }
    }
}

// This entry point clears the entire screen to a default color and resets the depth buffer
// to the farthest possible depth. We run it before rasterizing geometry. The workgroup
// size is tuned to handle blocks of pixels in parallel.
@compute @workgroup_size(256, 1)
fn clear(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let width = u32(screen_dims.width);
    let height = u32(screen_dims.height);
    let total_pixels = width * height;

    // If we're out of range, we simply don't process anything.
    if idx >= total_pixels {
        return;
    }

    // Black screen clear to ensure we have a known initial color.
    output_buffer.data[idx] = rgb(0u, 0u, 0u);

    // Set the depth buffer to max so that the first drawn fragment always appears.
    atomicStore(&depth_buffer.depth[idx], 0xFFFFFFFFu);
}

// This entry point takes each set of three vertices and renders a triangle. The
// index increments in chunks of three, so we don't step out of the array bounds.
// Each triangle is then rasterized, producing pixels in the output buffer.
@compute @workgroup_size(256, 1)
fn raster(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_idx = global_id.x * 3u;

    // If we don't have enough vertices left in the buffer, we skip this thread.
    if triangle_idx + 2u >= arrayLength(&vertex_buffer.values) {
        return;
    }

    // We perform a projection on each vertex prior to rasterizing. This will
    // allow screen-space calculations and depth checking.
    let v1 = project(vertex_buffer.values[triangle_idx]);
    let v2 = project(vertex_buffer.values[triangle_idx + 1u]);
    let v3 = project(vertex_buffer.values[triangle_idx + 2u]);

    // Finally, we draw the triangle into our buffers (color + depth).
    draw_triangle(v1, v2, v3);
}