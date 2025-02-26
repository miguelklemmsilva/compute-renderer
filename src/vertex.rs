use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct WgpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

// struct requires padding to be a multiple of 16 bytes
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub _padding: f32,
    pub normal: [f32; 3],
    pub _padding2: f32,
    pub tex_coords: [f32; 2],
    pub padding: [f32; 2]
}

impl Default for GpuVertex {
    // auto-fill padding data with 0.0
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            _padding: 0.0,
            normal: [0.0; 3],
            _padding2: 0.0,
            tex_coords: [0.0; 2],
            padding: [0.0; 2],
        }
    }
}