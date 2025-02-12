use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct WgpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct GpuVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub tex_coords: [f32; 2],
    pub padding: [f32; 2]
}