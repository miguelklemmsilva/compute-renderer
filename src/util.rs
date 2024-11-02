use bytemuck::{Pod, Zeroable};

#[allow(clippy::iter_nth_zero)]
pub fn process_gltf_model(file: &str) -> Vec<Vertex> {
    let (model, buffers, _) = {
        let bytes = std::fs::read(file).unwrap();
        gltf::import_slice(bytes).unwrap()
    };
    let mesh = model.meshes().nth(0).unwrap();
    let primitives = mesh.primitives().nth(0).unwrap();
    let reader = primitives.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions = reader.read_positions().unwrap().collect::<Vec<_>>();
    reader
        .read_indices()
        .unwrap()
        .into_u32()
        .map(|i| Vertex::from(positions[i as usize]))
        .collect()
}

pub fn process_obj_model(file: &str) -> Vec<Vertex> {
    let obj_data = obj::ObjData::load_buf(&mut std::fs::File::open(file).unwrap()).unwrap();
    let mut vertices = Vec::new();

    // Loop over each face and add vertices for each vertex position in the face
    for face in obj_data.objects[0].groups[0].polys.iter() {
        for index in &face.0 {
            let position = obj_data.position[index.0];
            vertices.push(Vertex::from(position));
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
    v: [f32; 3],
}

impl Vertex {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { v: [x, y, z] }
    }
}

macro_rules! v {
    ($x:expr, $y:expr, $z:expr) => {
        Vertex::new($x, $y, $z)
    };
}
pub(crate) use v;

impl From<[f32; 3]> for Vertex {
    fn from(v: [f32; 3]) -> Self {
        v!(v[0], v[1], v[2])
    }
}