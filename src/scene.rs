use crate::{
    camera::{self, Camera},
    gpu,
    model::{self, Material, Model, Texture},
    util::process_obj_model,
};

pub struct Scene {
    pub models: Vec<Model>,
    cameras: Vec<Camera>,
    active_camera: Option<usize>,
    pub materials: Vec<Material>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            models: vec![],
            cameras: vec![],
            active_camera: None,
            materials: vec![],
        }
    }

    /// Adds a model and returns a handle (index or reference) for easier access later
    pub fn add_model(&mut self, model_file: &str) -> usize {
        let vertices = process_obj_model(model_file);
        let mut model = Model { vertices };
        model.without_texture();
        self.models.push(model);
        self.models.len() - 1 // Returns the model index for easy access
    }

    /// Adds a texture to the scene and applies it to the specified model
    pub fn add_texture_to_model(&mut self, model_index: usize, texture_file: &str) {
        let texture = Texture::load(texture_file);
        let texture_index = self.materials.len() as u32;

        self.materials.push(Material {
            texture,
            texture_index,
        });

        // Apply texture to the model (assuming one texture per model in this example)
        if let Some(model) = self.models.get_mut(model_index) {
            model.apply_texture(texture_index);
        }
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