use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct WgpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl WgpuVertex {
    /// Returns the wgpu layout describing how the `Vertex` is laid out in memory
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: 12 as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: 24 as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
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