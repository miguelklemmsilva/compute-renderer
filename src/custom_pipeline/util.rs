use bytemuck::{Pod, Zeroable};

use super::raster_pass::TILE_SIZE;

pub(crate) const WORKGROUP_SIZE: u32 = 256;
pub(crate) const fn dispatch_size(len: u32) -> u32 {
    let subgroup_size = WORKGROUP_SIZE;
    (len + subgroup_size - 1) / subgroup_size
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ScreenUniform {
    screen_width: f32,
    screen_height: f32,
    num_tiles_x: u32,
    num_tiles_y: u32,
}

impl ScreenUniform {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            num_tiles_x: (screen_width as u32 + TILE_SIZE - 1) / TILE_SIZE,
            num_tiles_y: (screen_height as u32 + TILE_SIZE - 1) / TILE_SIZE,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Index(pub u32);

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

impl Default for MaterialInfo {
    fn default() -> Self {
        Self {
            texture_info: TextureInfo::default(),
            ambient: [1.0, 1.0, 1.0],
            _padding1: 0.0,
            specular: [0.0; 3],
            _padding2: 0.0,
            diffuse: [0.5, 0.5, 0.5],
            shininess: 0.0,
            dissolve: 1.0,
            optical_density: 0.0,
            _padding3: [0.0; 2],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TextureInfo {
    pub offset: u32,
    pub width: u32,
    pub height: u32,
    pub _padding: u32,
}

impl Default for TextureInfo {
    fn default() -> Self {
        Self {
            offset: u32::MAX,
            width: 0,
            height: 0,
            _padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Fragment {
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub world_pos: [f32; 3],
    pub padding: [f32; 4],
}

pub fn create_buffer_bind_group_layout_entry(
    index: u32,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: index,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
