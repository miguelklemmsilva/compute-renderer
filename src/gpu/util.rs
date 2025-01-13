use bytemuck::{Pod, Zeroable};

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
pub struct Index(pub u32);

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub texture_index: u32,
    pub w_clip: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TextureInfo {
    pub offset: u32,
    pub width: u32,
    pub height: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Fragment {
    pub depth: u32,
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub world_pos: [f32; 3],
    pub texture_index: u32,
    pub _padding: [u32; 2],
}

pub fn process_obj_model(file: &str) -> (Vec<Vertex>, Vec<Index>) {
    let (models, _) = tobj::load_obj(
        file,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    let mut final_vertices = Vec::new();
    let mut final_indices = Vec::new();
    let mut global_vertex_offset = 0;

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty() {
                        Vertex {
                            position: [
                                m.mesh.positions[3 * i],
                                m.mesh.positions[3 * i + 1],
                                m.mesh.positions[3 * i + 2],
                            ],
                            tex_coords: if !m.mesh.texcoords.is_empty() {
                                [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                            } else {
                                [0.0, 0.0] // Default tex_coords if missing
                            },
                            normal: [0.0, 1.0, 0.0],
                            texture_index: u32::MAX,
                            w_clip: 0.0,
                        }
                    } else {
                        Vertex {
                            position: [
                                m.mesh.positions[3 * i],
                                m.mesh.positions[3 * i + 1],
                                m.mesh.positions[3 * i + 2],
                            ],
                            tex_coords: if !m.mesh.texcoords.is_empty() {
                                [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                            } else {
                                [0.0, 0.0] // Default tex_coords if missing
                            },
                            normal: [
                                m.mesh.normals[3 * i],
                                m.mesh.normals[3 * i + 1],
                                m.mesh.normals[3 * i + 2],
                            ],
                            texture_index: u32::MAX,
                            w_clip: 0.0,
                        }
                    }
                })
                .collect::<Vec<_>>();

            final_vertices.extend(vertices);
            final_indices.extend(
                m.mesh
                    .indices
                    .iter()
                    .map(|i| Index(i + global_vertex_offset)),
            );
            global_vertex_offset += m.mesh.positions.len() as u32 / 3;
        })
        .collect::<Vec<_>>();

    (final_vertices, final_indices)
}
