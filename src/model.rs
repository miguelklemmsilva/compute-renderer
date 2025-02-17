use std::{fs::File, io::BufReader};

use crate::{
    custom_pipeline::util::Index,
    util::get_asset_path,
    vertex::{GpuVertex, WgpuVertex},
    window::BackendType,
};

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub processed_vertices_custom: Vec<GpuVertex>,
    pub processed_vertices_wgpu: Vec<WgpuVertex>,
    pub processed_indices: Vec<Index>,
}

impl Model {
    pub async fn new(file_name: &str, backend_type: BackendType) -> Model {
        // 1) Load OBJ text
        let obj_text = get_asset_path(file_name);
        let directory = obj_text.parent().unwrap();
        let mut obj_reader = BufReader::new(File::open(obj_text.as_path()).unwrap());

        // 2) tobj async: loads .obj + .mtl
        let (m, _m_materials) = tobj::load_obj_buf(
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
        let mut meshes = Vec::new();

        // Keep track of vertex count for index offsetting
        let mut current_vertex_count = 0;

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
                            ..Default::default()
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

        Model {
            meshes,
            processed_vertices_custom: processed_vertices_gpu,
            processed_vertices_wgpu,
            processed_indices,
        }
    }
}

pub struct Mesh {
    pub indices: Vec<Index>,
}
