use std::u32;

use bytemuck::{Pod, Zeroable};

pub fn process_obj_model(file: &str) -> Vec<Vertex> {
    let (models, materials) = tobj::load_obj(
        file,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    let mut vertices = vec![];
    for model in models.iter() {
        let mesh = &model.mesh;

        for &index in &mesh.indices {
            let index = index as usize;
            let pos_index = 3 * index;
            let tex_index = 2 * index;

            let position = [
                mesh.positions[pos_index],
                mesh.positions[pos_index + 1],
                mesh.positions[pos_index + 2],
            ];

            let tex_coords = if !mesh.texcoords.is_empty() {
                [mesh.texcoords[tex_index], mesh.texcoords[tex_index + 1]]
            } else {
                [0.0, 0.0]
            };

            let material_index = u32::MAX;

            vertices.push(Vertex {
                position,
                tex_coords,
                texture_index: material_index,
            });
        }
    }

    // print amount of vertices
    println!("Amount of vertices: {}", vertices.len());

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
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub texture_index: u32,
}
