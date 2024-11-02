use crate::util::{ process_obj_model, Vertex};

pub struct Model {
    pub vertices: Vec<Vertex>,
}

impl Model {
    pub fn new(filename: &str) -> Model {
        Model {
            vertices: process_obj_model(filename),
        }
    }
}
