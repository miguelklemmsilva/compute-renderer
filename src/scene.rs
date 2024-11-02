use crate::{camera, gpu, model};

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

    pub fn update(&mut self, gpu: &mut gpu::GPU) {
        if let Some(camera) = self.get_active_camera_mut() {
            // The camera is already updated in the main loop
            // Update the CameraUniform with the new view-projection matrix
            let mut camera_uniform = camera::CameraUniform::default();
            camera_uniform.update_view_proj(camera);

            // Write the updated CameraUniform data to the GPU buffer
            gpu.queue
                .write_buffer(&gpu.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        }
    }

    pub fn add_camera(&mut self, camera: camera::Camera) {
        self.cameras.push(camera);
    }

    pub fn get_active_camera_mut(&mut self) -> Option<&mut camera::Camera> {
        self.active_camera
            .and_then(move |index| self.cameras.get_mut(index))
    }

    pub fn add_model(&mut self, model: model::Model) {
        self.models.push(model);
    }

    pub fn get_active_camera(&self) -> Option<camera::Camera> {
        self.active_camera.map(|index| self.cameras[index])
    }

    pub fn set_active_camera(&mut self, index: usize) {
        self.active_camera = Some(index);
    }
}
