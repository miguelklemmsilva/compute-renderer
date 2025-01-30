// Example vertex & fragment for demonstration.
// Your actual code can incorporate your lighting, effect, etc.

// The bind group has the camera buffer at binding(0), the light buffer at binding(1),
// and the effect buffer at binding(2). Adjust the struct layouts as you see fit.

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

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> lights: array<Light, 8>; // or however many you want

// Vertex inputs
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

// Vertex outputs
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to world space and clip space
    let world_pos = vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz;
    
    // Pass world normal
    out.world_normal = normalize(in.normal);
    out.uv = in.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    
    // Basic lighting calculation
    var final_color = vec3<f32>(0.0);
    let ambient = vec3<f32>(0.1);
    
    // Start with ambient light
    final_color = ambient;
    
    // Add contribution from each light
    for (var i = 0u; i < 8u; i++) {
        let light = lights[i];
        let light_dir = normalize(light.world_position - in.world_position);
        
        // Diffuse
        let diff = max(dot(normal, light_dir), 0.0);
        
        // Specular
        let view_dir = normalize(camera.view_position.xyz - in.world_position);
        let reflect_dir = reflect(-light_dir, normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);

        final_color += (diff + spec * 0.5) * light.color * light.intensity;
    }
    
    // Ensure the color doesn't exceed 1.0
    final_color = min(final_color, vec3<f32>(1.0));

    return vec4<f32>(final_color, 1.0);
}