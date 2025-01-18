use std::{
    fs::File,
    io::{BufReader, Cursor},
};

use crate::{
    gpu::util::{Index, Vertex},
    util::get_asset_path,
};

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_color: [f32; 3],
    pub diffuse_texture: Option<Texture>,
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
    pub dissolve: f32,
    pub optical_density: f32,
}

impl Model {
    pub async fn new(file_name: &str) -> Model {
        // 1) Load OBJ text
        let obj_text = get_asset_path(file_name);
        let directory = obj_text.parent().unwrap();
        let mut obj_reader = BufReader::new(File::open(obj_text.as_path()).unwrap());

        // 2) tobj async: loads .obj + .mtl
        let (m, m_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                let mat_text = File::open(directory.join(p.to_path_buf())).unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(mat_text))
            },
        )
        .expect("Failed to load model");

        // 3) Build our Material array
        let mut materials = Vec::new();
        println!("{:?}", m_materials);

        for m in m_materials.unwrap() {
            materials.push(Material {
                name: m.name.clone(),
                diffuse_color: m.diffuse.unwrap_or([0.0, 0.0, 0.0]),
                diffuse_texture: m
                    .diffuse_texture
                    .as_ref()
                    .map(|texture| Texture::load(&directory.join(texture).to_str().unwrap())),
                ambient: m.ambient.unwrap_or([0.0, 0.0, 0.0]),
                specular: m.specular.unwrap_or([0.0, 0.0, 0.0]),
                shininess: m.shininess.unwrap_or(0.0),
                dissolve: m.dissolve.unwrap_or(0.0),
                optical_density: m.optical_density.unwrap_or(0.0),
            });
        }

        let meshes = m
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| {
                        if m.mesh.normals.is_empty() {
                            Vertex {
                                position: [
                                    m.mesh.positions[i * 3],
                                    m.mesh.positions[i * 3 + 1],
                                    m.mesh.positions[i * 3 + 2],
                                ],
                                tex_coords: if m.mesh.texcoords.is_empty() {
                                    [0.0, 0.0]
                                } else {
                                    [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                                },
                                normal: [0.0, 0.0, 0.0],
                                material_id: m.mesh.material_id.unwrap_or(0) as u32,
                                w_clip: 0.0,
                            }
                        } else {
                            Vertex {
                                position: [
                                    m.mesh.positions[i * 3],
                                    m.mesh.positions[i * 3 + 1],
                                    m.mesh.positions[i * 3 + 2],
                                ],
                                tex_coords: if m.mesh.texcoords.is_empty() {
                                    [0.0, 0.0]
                                } else {
                                    [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                                },
                                normal: [
                                    m.mesh.normals[i * 3],
                                    m.mesh.normals[i * 3 + 1],
                                    m.mesh.normals[i * 3 + 2],
                                ],
                                material_id: m.mesh.material_id.unwrap_or(0) as u32,
                                w_clip: 0.0,
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                // material_id is how we know which Material to use
                Mesh {
                    name: file_name.to_string(),
                    vertices,
                    indices: m.mesh.indices.into_iter().map(|i| Index(i)).collect(),
                }
            })
            .collect::<Vec<_>>();

        Model { meshes, materials }
    }
}

pub struct Texture {
    pub data: Vec<u32>,
    pub width: u32,
    pub height: u32,
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

    pub fn default() -> Texture {
        Texture {
            data: vec![0],
            width: 1,
            height: 1,
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
}
