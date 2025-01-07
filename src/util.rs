use std::path::{Path, PathBuf};
use std::u32;

use bytemuck::{Pod, Zeroable};

pub fn process_obj_model(file: &str) -> Vec<Vertex> {
    let (models, _) = tobj::load_obj(
        file,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    let mut vertices = Vec::with_capacity(models.iter().map(|m| m.mesh.indices.len()).sum());
    for model in models.iter() {
        let mesh = &model.mesh;

        let has_texcoords = !mesh.texcoords.is_empty();
        let has_normals = !mesh.normals.is_empty();

        for &index in &mesh.indices {
            let idx = index as usize;

            vertices.push(Vertex {
                position: [
                    mesh.positions[3 * idx],
                    mesh.positions[3 * idx + 1],
                    mesh.positions[3 * idx + 2],
                ],
                tex_coords: if has_texcoords {
                    [mesh.texcoords[2 * idx], mesh.texcoords[2 * idx + 1]]
                } else {
                    [0.0, 0.0]
                },
                normal: if has_normals {
                    [
                        mesh.normals[3 * idx],
                        mesh.normals[3 * idx + 1],
                        mesh.normals[3 * idx + 2],
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                },
                texture_index: u32::MAX,
                w_clip: 0.0,
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

pub fn get_asset_path(asset: &str) -> PathBuf {
    // First, try looking for assets relative to the executable
    let executable_path = std::env::current_exe().expect("Failed to get executable path");
    let executable_dir = executable_path
        .parent()
        .expect("Failed to get executable directory");

    // Check different possible asset locations
    let possible_paths = vec![
        // 1. Check next to the executable
        executable_dir.join("assets").join(asset),
        // 2. Check in Resources folder (for macOS .app bundles)
        executable_dir.join("../Resources/assets").join(asset),
        // 3. Check relative to CARGO_MANIFEST_DIR (for development)
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(asset),
    ];

    // Try each path and return the first one that exists
    for path in possible_paths {
        if path.exists() {
            return path;
        }
    }

    panic!("Could not find asset: {}", asset);
}
