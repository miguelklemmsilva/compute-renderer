use crate::{camera, model};

pub struct Scene {
    pub models: Vec<model::Model>,
    cameras: Vec<camera::Camera>,
}

impl Scene {
    pub fn new() -> Scene {
    Scene {
        models: vec![],
        cameras: vec![],
    }
}
    pub fn add_camera(&mut self, camera: camera::Camera) {
        self.cameras.push(camera);
    }

    pub fn add_model(&mut self, model: model::Model) {
        self.models.push(model);
    }
}