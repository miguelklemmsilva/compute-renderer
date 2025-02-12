use std::{collections::HashMap, fs::File, io::BufReader};

use crate::{
    custom_pipeline::util::{Index, MaterialInfo, TextureInfo},
    util::get_asset_path,
    vertex::{GpuVertex, WgpuVertex},
    window::BackendType,
};

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub processed_vertices_custom: Vec<GpuVertex>,
    pub processed_vertices_wgpu: Vec<WgpuVertex>,
    pub processed_indices: Vec<Index>,
    pub processed_materials: Vec<MaterialInfo>,
    pub processed_textures: Vec<u32>,
}

pub struct Material {
    pub diffuse_color: [f32; 3],
    pub diffuse_texture: Option<Texture>,
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
    pub dissolve: f32,
    pub optical_density: f32,
}

impl Model {
    pub async fn new(file_name: &str, backend_type: BackendType) -> Model {
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
                let mat_text = File::open(directory.join(p.to_path_buf()));
                if let Ok(mat_text) = mat_text {
                    tobj::load_mtl_buf(&mut BufReader::new(mat_text))
                } else {
                    Err(tobj::LoadError::OpenFileFailed)
                }
            },
        )
        .expect("Failed to load model");

        // Pre-allocate vectors for processed data
        let mut processed_vertices_gpu = Vec::new();
        let mut processed_vertices_wgpu = Vec::new();
        let mut processed_indices = Vec::new();
        let mut processed_materials = Vec::new();
        let mut processed_textures = Vec::new();
        let mut meshes = Vec::new();

        // Track unique textures to avoid duplication
        let mut texture_map: HashMap<String, (u32, u32, u32)> = HashMap::new(); // path -> (offset, width, height)

        // Keep track of vertex count for index offsetting
        let mut current_vertex_count = 0;

        if let Ok(m_materials) = m_materials {
            // Process materials first
            for m in m_materials {
                // Create the material
                let material = Material {
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
                };

                // Process material for GPU
                const NO_TEXTURE_INDEX: u32 = 0xFFFFFFFF;
                let texture_info = if let Some(tex) = &material.diffuse_texture {
                    let texture_path = m.diffuse_texture.as_ref().unwrap().to_string();

                    // Check if we've already processed this texture
                    let (offset, width, height) =
                        if let Some(&cached) = texture_map.get(&texture_path) {
                            cached
                        } else {
                            // If not, add it to our processed textures and cache the info
                            let offset = processed_textures.len() as u32;
                            processed_textures.extend_from_slice(&tex.data);
                            let info = (offset, tex.width, tex.height);
                            texture_map.insert(texture_path, info);
                            info
                        };

                    TextureInfo {
                        offset,
                        width,
                        height,
                        _padding: 0,
                    }
                } else {
                    TextureInfo {
                        offset: NO_TEXTURE_INDEX,
                        width: 0,
                        height: 0,
                        _padding: 0,
                    }
                };

                let material_info = MaterialInfo {
                    texture_info,
                    ambient: material.ambient,
                    _padding1: 0.0,
                    specular: material.specular,
                    _padding2: 0.0,
                    diffuse: material.diffuse_color,
                    shininess: material.shininess,
                    dissolve: material.dissolve,
                    optical_density: material.optical_density,
                    _padding3: [0.0, 0.0],
                };

                processed_materials.push(material_info);
            }
        }

        if processed_textures.is_empty() && processed_materials.is_empty() {
            processed_textures.push(0);
            processed_materials.push(MaterialInfo::default());
        }

        // Process meshes and their vertices/indices
        for m in m {
            match backend_type {
                BackendType::CustomPipeline => {
                    let vertices = (0..m.mesh.positions.len() / 3)
                        .map(|i| GpuVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                                1.0
                            ],
                            tex_coords: if m.mesh.texcoords.is_empty() {
                                [0.0, 0.0]
                            } else {
                                [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                            },
                            normal: if m.mesh.normals.is_empty() {
                                [0.0, 0.0, 0.0, 0.0]
                            } else {
                                [
                                    m.mesh.normals[i * 3],
                                    m.mesh.normals[i * 3 + 1],
                                    m.mesh.normals[i * 3 + 2],
                                    0.0
                                ]
                            },
                            padding: [0.0; 2]
                        })
                        .collect::<Vec<_>>();
                    processed_vertices_gpu.extend(vertices);
                }
                BackendType::WgpuPipeline => {
                    let vertices = (0..m.mesh.positions.len() / 3)
                        .map(|i| WgpuVertex {
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
                            normal: if m.mesh.normals.is_empty() {
                                [0.0, 0.0, 0.0]
                            } else {
                                [
                                    m.mesh.normals[i * 3],
                                    m.mesh.normals[i * 3 + 1],
                                    m.mesh.normals[i * 3 + 2],
                                ]
                            },
                        })
                        .collect::<Vec<_>>();
                    processed_vertices_wgpu.extend(vertices);
                }
            }

            // Process indices with correct offset
            let indices: Vec<Index> = m
                .mesh
                .indices
                .iter()
                .map(|&i| Index(i + current_vertex_count))
                .collect();

            // Store the mesh
            meshes.push(Mesh {
                indices: indices.clone(),
            });

            // Update processed data
            processed_indices.extend(indices);
            current_vertex_count = match backend_type {
                BackendType::CustomPipeline => processed_vertices_gpu.len() as u32,
                BackendType::WgpuPipeline => processed_vertices_wgpu.len() as u32,
            };
        }

        // If no textures exist, use a small fallback
        if processed_textures.is_empty() {
            processed_textures.push(0);
        }

        Model {
            meshes,
            processed_vertices_custom: processed_vertices_gpu,
            processed_vertices_wgpu,
            processed_indices,
            processed_materials,
            processed_textures,
        }
    }
}

pub struct Texture {
    pub data: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn load(filename: &str) -> Texture {
        // instead of crashing if the texture is not found return empty texture
        let img = match image::open(filename) {
            Ok(img) => img.to_rgba8(),
            Err(_) => return Texture::default(),
        };
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
    pub indices: Vec<Index>,
}
