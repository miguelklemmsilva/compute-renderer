use crate::util::{process_gltf_model, process_obj_model, Vertex};

pub enum FileType {
    Obj,
    Gltf,
}

pub struct Model {
    pub vertices: Vec<Vertex>,
}

impl Model {
    pub fn new(filename: &str, filetype: FileType) -> Model {
        Model {
            vertices: match filetype {
                FileType::Obj => process_obj_model(filename),
                FileType::Gltf => process_gltf_model(filename),
            },
        }
    }
}
