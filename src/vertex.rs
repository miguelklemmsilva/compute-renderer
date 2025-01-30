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
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub material_id: u32,
    pub w_clip: f32,
}

impl From<WgpuVertex> for GpuVertex {
    fn from(v: WgpuVertex) -> Self {
        Self {
            position: v.position,
            normal: v.normal,
            tex_coords: v.tex_coords,
            material_id: 0,
            w_clip: 0.0,
        }
    }
}

impl From<GpuVertex> for WgpuVertex {
    fn from(v: GpuVertex) -> Self {
        Self {
            position: v.position,
            normal: v.normal,
            tex_coords: v.tex_coords,
        }
    }
}
