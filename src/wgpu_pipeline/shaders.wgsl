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
var<storage, read> lights: array<Light>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.position = world_pos.xyz;
    out.normal = normalize(in.normal);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    
    // Start with the same ambient light as in your compute shader.
    var final_color = vec3<f32>(0.0);

    let num_lights = arrayLength(&lights);
    for (var i = 0u; i < num_lights; i++) {
        let light = lights[i];
        let light_dir = normalize(light.world_position - in.position);
        let diff = max(dot(normal, light_dir), 0.0);
        let view_dir = normalize(camera.view_position.xyz - in.position);
        let reflect_dir = reflect(-light_dir, normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        final_color += (diff + spec * 0.5) * light.color * light.intensity;
    }
    
    final_color = min(final_color, vec3<f32>(1.0));
    return vec4<f32>(final_color, 1.0);
}