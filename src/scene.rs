use crate::{
    camera::{self, Camera},
    gpu,
    model::{self, Material, Model},
    util::process_obj_model,
};

pub struct Scene {
    pub models: Vec<Model>,
    cameras: Vec<Camera>,
    active_camera: Option<usize>,
    pub materials: Vec<Material>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            models: vec![],
            cameras: vec![],
            active_camera: None,
            materials: vec![],
        }
    }

    pub fn add_model(&mut self, model_file: &str) -> &mut Model {
        let mut vertices = process_obj_model(model_file);

        // Set the texture_index to u32::MAX to indicate no texture
        for vertex in &mut vertices {
            vertex.texture_index = u32::MAX;
        }

        let model = Model { vertices };
        self.models.push(model);
        self.models.last_mut().unwrap()
    }

    pub fn add_texture(&mut self, texture_file: &str) -> u32 {
        let texture = model::Texture::load(texture_file);
        let texture_index = self.materials.len() as u32;

        let material = Material {
            texture,
            texture_index,
        };
        self.materials.push(material);

        texture_index
    }

    pub fn add_camera(&mut self, camera: camera::Camera) {
        self.cameras.push(camera);
    }

    pub fn set_active_camera(&mut self, index: usize) {
        self.active_camera = Some(index);
    }

    pub fn get_active_camera_mut(&mut self) -> Option<&mut camera::Camera> {
        self.active_camera
            .and_then(move |index| self.cameras.get_mut(index))
    }

    pub fn get_active_camera(&self) -> Option<camera::Camera> {
        self.active_camera.map(|index| self.cameras[index])
    }

    pub fn update(&mut self, gpu: &mut gpu::GPU) {
        if let Some(camera) = self.get_active_camera_mut() {
            let mut camera_uniform = camera::CameraUniform::default();
            camera_uniform.update_view_proj(camera);
            gpu.queue
                .write_buffer(&gpu.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        }
    }
}