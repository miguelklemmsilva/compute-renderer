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
    obj::ObjData::load_buf(&mut std::fs::File::open(file).unwrap())
        .unwrap()
        .position
        .iter()
        .cloned()
        .map(Vertex::from)
        .collect()
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
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    v: [f32; 3],
}

impl Vertex {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as _;
    pub const ATTR: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

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