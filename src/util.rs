use bytemuck::{Pod, Zeroable};

pub fn process_obj_model(file: &str) -> Vec<Vertex> {
    let obj_data = obj::ObjData::load_buf(&mut std::fs::File::open(file).unwrap()).unwrap();
    let mut vertices = Vec::new();

    for object in &obj_data.objects {
        for group in &object.groups {
            for poly in &group.polys {
                for index in &poly.0 {
                    let position = obj_data.position[index.0];
                    let tex_coord = if let Some(tex_idx) = index.1 {
                        obj_data.texture[tex_idx]
                    } else {
                        [0.0, 0.0] // Default UV if none provided
                    };
                    vertices.push(Vertex::from((position, tex_coord)));
                }
            }
        }
    }

    vertices
}

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
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct Vertex {
    position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    pub const fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
        Self {
            position: [x, y, z],
            tex_coords: [u, v],
        }
    }
}

impl From<([f32; 3], [f32; 2])> for Vertex {
    fn from(data: ([f32; 3], [f32; 2])) -> Self {
        Vertex::new(data.0[0], data.0[1], data.0[2], data.1[0], data.1[1])
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TextureDims {
    pub width: u32,
    pub height: u32,
}