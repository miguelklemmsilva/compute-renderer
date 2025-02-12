struct CameraUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Light {
    world_position: vec3<f32>,
    _pad1: f32,
    view_position: vec3<f32>,
    _pad2: f32,
    color: vec3<f32>,
    intensity: f32,
};

struct ScreenUniform {
    width: f32,
    height: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;
@group(0) @binding(1)
var<uniform> screen_dims: ScreenUniform;
@group(0) @binding(2)
var<storage, read> lights: array<Light>;

// Vertex inputs
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

// Vertex outputs
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Fragment {
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Define three vertices that cover the entire screen.
    // This “oversized” triangle is a common trick for full–screen passes.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    // Compute UV coordinates so the output texture is mapped correctly.
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 2.0)
    );
    let pos = positions[vertex_index];
    let uv = uvs[vertex_index];
    return VertexOutput(vec4<f32>(pos, 0.0, 1.0), uv);
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    
    // Basic lighting calculation
    var final_color = vec3<f32>(0.0);
    let ambient = vec3<f32>(0.1);
    
    // Start with ambient light
    final_color = ambient;

    let num_lights = arrayLength(&lights);
    
    // Add contribution from each light
    for (var i = 0u; i < num_lights; i++) {
        let light = lights[i];
        let light_dir = normalize(light.world_position - in.world_pos);
        
        // Diffuse
        let diff = max(dot(normal, light_dir), 0.0);
        
        // Specular
        let view_dir = normalize(camera.view_position.xyz - in.world_pos);
        let reflect_dir = reflect(-light_dir, normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);

        final_color += (diff + spec * 0.5) * light.color * light.intensity;
    }
    
    // Ensure the color doesn't exceed 1.0
    final_color = min(final_color, vec3<f32>(1.0));

    return vec4<f32>(final_color, 1.0);
}