use bytemuck::{Pod, Zeroable};

pub(crate) const WORKGROUP_SIZE: u32 = 256;
pub(crate) const fn dispatch_size(len: u32) -> u32 {
    let subgroup_size = WORKGROUP_SIZE;
    (len + subgroup_size - 1) / subgroup_size
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Uniform {
    screen_width: f32,
    screen_height: f32,
}

impl Uniform {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Index(pub u32);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub material_id: u32,
    pub w_clip: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct MaterialInfo {
    pub texture_info: TextureInfo,
    pub ambient: [f32; 3],
    pub _padding1: f32,
    pub specular: [f32; 3],
    pub _padding2: f32,
    pub diffuse: [f32; 3],
    pub shininess: f32,
    pub dissolve: f32,
    pub optical_density: f32,
    pub _padding3: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TextureInfo {
    pub offset: u32,
    pub width: u32,
    pub height: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Fragment {
    pub depth: u32,
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub world_pos: [f32; 3],
    pub texture_index: u32,
    pub _padding: [u32; 2],
}
