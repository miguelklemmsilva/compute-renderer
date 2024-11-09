use std::u32;

use bytemuck::{Pod, Zeroable};

fn compute_face_normal(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    let u = [
        v1[0] - v0[0],
        v1[1] - v0[1],
        v1[2] - v0[2],
    ];
    let v = [
        v2[0] - v0[0],
        v2[1] - v0[1],
        v2[2] - v0[2],
    ];

    let normal = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];

    normal
}

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

        // Map to accumulate normals per vertex
        use std::collections::HashMap;
        let mut vertex_normals: HashMap<usize, [f32; 3]> = HashMap::new();

        // Initialize normals
        for i in 0..mesh.positions.len() / 3 {
            vertex_normals.insert(i, [0.0, 0.0, 0.0]);
        }

        // Compute normals if they are missing
        if mesh.normals.is_empty() {
            for face in mesh.indices.chunks(3) {
                let i0 = face[0] as usize;
                let i1 = face[1] as usize;
                let i2 = face[2] as usize;

                let v0 = [
                    mesh.positions[3 * i0],
                    mesh.positions[3 * i0 + 1],
                    mesh.positions[3 * i0 + 2],
                ];
                let v1 = [
                    mesh.positions[3 * i1],
                    mesh.positions[3 * i1 + 1],
                    mesh.positions[3 * i1 + 2],
                ];
                let v2 = [
                    mesh.positions[3 * i2],
                    mesh.positions[3 * i2 + 1],
                    mesh.positions[3 * i2 + 2],
                ];

                // Compute face normal
                let normal = compute_face_normal(v0, v1, v2);

                // Accumulate normals
                for &i in &[i0, i1, i2] {
                    let n = vertex_normals.get_mut(&i).unwrap();
                    n[0] += normal[0];
                    n[1] += normal[1];
                    n[2] += normal[2];
                }
            }

            // Normalize accumulated normals
            for normal in vertex_normals.values_mut() {
                let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
                if length > 0.0 {
                    normal[0] /= length;
                    normal[1] /= length;
                    normal[2] /= length;
                }
            }
        }

        for &index in &mesh.indices {
            let index = index as usize;
            let pos_index = 3 * index;
            let tex_index = 2 * index;
            let normal_index = 3 * index;

            let position = [
                mesh.positions[pos_index],
                mesh.positions[pos_index + 1],
                mesh.positions[pos_index + 2],
            ];

            let normal = if !mesh.normals.is_empty() {
                [
                    mesh.normals[normal_index],
                    mesh.normals[normal_index + 1],
                    mesh.normals[normal_index + 2],
                ]
            } else {
                vertex_normals.get(&index).cloned().unwrap_or([0.0, 0.0, 0.0])
            };

            let tex_coords = if !mesh.texcoords.is_empty() {
                [mesh.texcoords[tex_index], mesh.texcoords[tex_index + 1]]
            } else {
                [0.0, 0.0]
            };

            let material_index = u32::MAX;

            vertices.push(Vertex {
                position,
                normal,
                tex_coords,
                texture_index: material_index,
            });
        }
    }

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
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub texture_index: u32,
}
