use crate::gpu::util::{Index, Vertex};

pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
}

impl Model {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self { vertices, indices }
    }
}

pub struct Texture {
    pub data: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

pub struct Material {
    pub texture: Texture,
    pub texture_index: u32,
}

impl Texture {
    pub fn load(filename: &str) -> Texture {
        let img = image::open(filename).unwrap().to_rgba8();
        let (width, height) = img.dimensions();
        let raw_data = img.into_raw();

        let data = raw_data
            .chunks_exact(4)
            .map(|chunk| {
                let r = chunk[0] as u32;
                let g = chunk[1] as u32;
                let b = chunk[2] as u32;
                let a = chunk[3] as u32;
                (r << 24) | (g << 16) | (b << 8) | a
            })
            .collect();

        Texture {
            data,
            width,
            height,
        }
    }
}
