use crate::{camera, model};

pub struct Scene {
    pub models: Vec<model::Model>,
    cameras: Vec<camera::Camera>,
    active_camera: Option<usize>,
}

impl Scene {
    pub fn new() -> Scene {
    Scene {
        models: vec![],
        cameras: vec![],
        active_camera: Option::None,
    }
}
    pub fn add_camera(&mut self, camera: camera::Camera) {
        self.cameras.push(camera);
    }

    pub fn add_model(&mut self, model: model::Model) {
        self.models.push(model);
    }

    pub fn get_active_camera(&self) -> Option<&camera::Camera> {
        self.active_camera.map(|index| &self.cameras[index])
    }

    pub fn set_active_camera(&mut self, index: usize) {
        self.active_camera = Some(index);
    }
}