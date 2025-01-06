use crate::model::{Material, Model, Texture};
use crate::{camera, effect::Effect, gpu, util::process_obj_model};
use std::time::Duration;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    pub world_position: [f32; 3],
    _padding1: f32,
    pub view_position: [f32; 3],
    _padding2: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            world_position: [5.0, 5.0, 5.0],
            _padding1: 0.0,
            view_position: [0.0, 0.0, 0.0],
            _padding2: 0.0,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
        }
    }
}

pub struct Scene {
    pub models: Vec<Model>,
    cameras: Vec<camera::Camera>,
    active_camera: Option<usize>,
    pub materials: Vec<Material>,
    pub lights: Vec<Light>,
    pub effects: Vec<Effect>,
    pub time: f32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            models: vec![],
            cameras: vec![],
            active_camera: None,
            materials: vec![],
            lights: vec![],
            effects: vec![],
            time: 0.0,
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
        let texture = Texture::load(texture_file);
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

    pub fn get_active_camera(&self) -> Option<&camera::Camera> {
        self.active_camera.and_then(|index| self.cameras.get(index))
    }

    pub fn update(&mut self, gpu: &mut gpu::gpu::GPU, delta_time: Duration) {
        self.time += delta_time.as_secs_f32();

        // Update effects only if there are any
        if !self.effects.is_empty() {
            for effect in &mut self.effects {
                effect.update(delta_time);
            }
        }

        // Update camera and get view matrix
        if let Some(camera) = self.get_active_camera() {
            let mut camera_uniform = camera::CameraUniform::default();
            camera_uniform.update_view_proj(camera);

            // Transform light positions to view space using only view matrix
            let view_matrix = camera.build_view_matrix();
            for light in &mut self.lights {
                let world_pos = glam::Vec3::from_slice(&light.world_position);
                let view_pos = view_matrix.transform_point3(world_pos);
                light.view_position = view_pos.to_array();
            }

            gpu.queue
                .write_buffer(&gpu.buffers.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        }

        // Update lights
        gpu.queue
            .write_buffer(&gpu.buffers.light_buffer, 0, bytemuck::cast_slice(&self.lights));

        // Update effects only if there are any
        if let Some(effect) = self.effects.first() {
            let mut effect_uniform = crate::effect::EffectUniform::default();
            effect_uniform.update(effect, self.time);
            gpu.queue
                .write_buffer(&gpu.buffers.effect_buffer, 0, bytemuck::bytes_of(&effect_uniform));
        } else {
            // Write a default "no effect" state
            let effect_uniform = crate::effect::EffectUniform::default();
            gpu.queue
                .write_buffer(&gpu.buffers.effect_buffer, 0, bytemuck::bytes_of(&effect_uniform));
        }
    }

    pub fn add_light(&mut self, position: [f32; 3], color: [f32; 3], intensity: f32) -> usize {
        let light = Light {
            world_position: position,
            _padding1: 0.0,
            view_position: [0.0, 0.0, 0.0],
            _padding2: 0.0,
            color,
            intensity,
        };
        self.lights.push(light);
        self.lights.len() - 1
    }

    pub fn update_light(
        &mut self,
        index: usize,
        position: Option<[f32; 3]>,
        color: Option<[f32; 3]>,
        intensity: Option<f32>,
    ) {
        if let Some(light) = self.lights.get_mut(index) {
            if let Some(pos) = position {
                light.world_position = pos;
            }
            if let Some(col) = color {
                light.color = col;
            }
            if let Some(int) = intensity {
                light.intensity = int;
            }
        }
    }

    pub fn get_lights(&self) -> &[Light] {
        &self.lights
    }

    pub fn add_effect(&mut self, effect: Effect) -> usize {
        self.effects.push(effect);
        self.effects.len() - 1
    }

    /// Duplicates a model multiple times for stress testing purposes
    /// Returns a vector of the new model indices
    pub fn duplicate_model_for_stress_test(
        &mut self,
        model_index: usize,
        count: usize,
        grid_spacing: f32,
    ) -> Vec<usize> {
        let mut new_indices = Vec::with_capacity(count);

        // Clone the original model's vertices first to avoid multiple borrows
        let vertices = if let Some(original_model) = self.models.get(model_index) {
            original_model.vertices.clone()
        } else {
            return new_indices;
        };

        // Calculate grid dimensions for a square-ish layout
        let grid_size = (count as f32).sqrt().ceil() as usize;

        // Now create new models with the cloned vertices
        for i in 0..count {
            let row = i / grid_size;
            let col = i % grid_size;

            // Calculate offset from center
            let x_offset = (col as f32 - (grid_size as f32 / 2.0)) * grid_spacing;
            let z_offset = (row as f32 - (grid_size as f32 / 2.0)) * grid_spacing;

            // Create new vertices with offset
            let mut new_vertices = vertices.clone();
            for vertex in &mut new_vertices {
                vertex.position[0] += x_offset;
                vertex.position[2] += z_offset;
            }

            let new_model = Model {
                vertices: new_vertices,
            };
            self.models.push(new_model);
            new_indices.push(self.models.len() - 1);
        }

        // Clear existing lights as we'll set up new ones for the stress test
        self.lights.clear();

        // Calculate the total size of the grid
        let total_width = grid_size as f32 * grid_spacing;
        let grid_height = 8.0; // Height of lights above the grid

        // Add a brighter central light above the scene
        self.add_light([0.0, grid_height, 0.0], [1.0, 1.0, 1.0], 2.0);

        // Add corner lights to ensure good coverage
        let corner_intensity = 1.5;
        let half_width = total_width / 2.0;

        // Add lights at each corner of the grid
        self.add_light(
            [half_width, grid_height, half_width],
            [1.0, 0.9, 0.8],
            corner_intensity,
        );
        self.add_light(
            [-half_width, grid_height, half_width],
            [1.0, 0.9, 0.8],
            corner_intensity,
        );
        self.add_light(
            [half_width, grid_height, -half_width],
            [1.0, 0.9, 0.8],
            corner_intensity,
        );
        self.add_light(
            [-half_width, grid_height, -half_width],
            [1.0, 0.9, 0.8],
            corner_intensity,
        );

        new_indices
    }
}

pub struct SceneConfig {
    pub name: String,
    pub model_path: String,
    pub texture_path: Option<String>,
    pub lights: Vec<(
        /* position */ [f32; 3],
        /* color */ [f32; 3],
        /* intensity */ f32,
    )>,
    pub effects: Option<Vec<Effect>>,
    // New stress test configuration
    pub stress_test: Option<StressTestConfig>,
    // Camera configuration
    pub camera_config: CameraConfig,
    // Benchmark duration in seconds
    pub benchmark_duration_secs: u64,
}

pub struct StressTestConfig {
    pub model_count: usize,
    pub grid_spacing: f32,
}

pub struct CameraConfig {
    pub distance: f32,
    pub theta: f32,
    pub phi: f32,
    pub target: [f32; 3],
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            distance: 3.0,
            theta: 0.0,
            phi: 0.0,
            target: [0.0, 0.0, 0.0],
        }
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            model_path: String::new(),
            texture_path: None,
            lights: Vec::new(),
            effects: None,
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        }
    }
}
