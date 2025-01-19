use crate::gpu::util::{MaterialInfo, TextureInfo};
use crate::model::{Material, Model, Texture};
use crate::{camera, effect::Effect, gpu};
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
            lights: vec![],
            effects: vec![],
            time: 0.0,
        }
    }

    /// Creates a new scene from a scene configuration
    pub async fn from_config(scene_config: &SceneConfig, width: usize, height: usize) -> Scene {
        let mut scene = Scene::new();

        let base_model = scene.add_obj_with_mtl(&scene_config.model_path).await;

        // If stress test is enabled, duplicate the model
        if let Some(stress_config) = &scene_config.stress_test {
            scene.duplicate_model_for_stress_test(
                base_model,
                stress_config.model_count,
                stress_config.grid_spacing,
            );
        } else {
            // Add lights from config if not a stress test
            for (position, color, intensity) in &scene_config.lights {
                scene.add_light(*position, *color, *intensity);
            }
        }

        // Add effects if specified
        if let Some(effects) = &scene_config.effects {
            for effect in effects {
                scene.add_effect(effect.clone());
            }
        }

        // Add camera and set active
        let camera = match scene_config.camera_config.mode {
            crate::camera::CameraMode::FirstPerson => crate::camera::Camera::new_first_person(
                glam::Vec3::from(scene_config.camera_config.position),
                (width as f32) / (height as f32),
            ),
            crate::camera::CameraMode::Orbit => crate::camera::Camera::new(
                scene_config.camera_config.distance,
                scene_config.camera_config.theta,
                scene_config.camera_config.phi,
                glam::Vec3::from(scene_config.camera_config.target),
                (width as f32) / (height as f32),
            ),
        };
        scene.add_camera(camera);
        scene.set_active_camera(0);

        scene
    }

    /// Adds an OBJ model *with MTL material(s)*, loads all textures,
    /// and sets up each sub-mesh's `texture_index` to point to the correct Material in `self.materials`.
    pub async fn add_obj_with_mtl(&mut self, obj_path: &str) -> usize {
        // (A) Load geometry + textures from the .obj + .mtl
        let model = Model::new(obj_path).await;
        self.models.push(model);
        self.models.len() - 1
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

            gpu.queue.write_buffer(
                &gpu.buffers.camera_buffer,
                0,
                bytemuck::bytes_of(&camera_uniform),
            );
        }

        // Update lights
        gpu.queue.write_buffer(
            &gpu.buffers.light_buffer,
            0,
            bytemuck::cast_slice(&self.lights),
        );

        // Update effects only if there are any
        if let Some(effect) = self.effects.first() {
            let mut effect_uniform = crate::effect::EffectUniform::default();
            effect_uniform.update(effect, self.time);
            gpu.queue.write_buffer(
                &gpu.buffers.effect_buffer,
                0,
                bytemuck::bytes_of(&effect_uniform),
            );
        } else {
            // Write a default "no effect" state
            let effect_uniform = crate::effect::EffectUniform::default();
            gpu.queue.write_buffer(
                &gpu.buffers.effect_buffer,
                0,
                bytemuck::bytes_of(&effect_uniform),
            );
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

        // Clone the original model's data first to avoid multiple borrows
        let processed_vertices = if let Some(original_model) = self.models.get(model_index) {
            original_model.processed_vertices.clone()
        } else {
            return new_indices;
        };

        // Get other necessary data from original model
        let original_model = &self.models[model_index];
        let processed_indices = original_model.processed_indices.clone();
        let processed_materials = original_model.processed_materials.clone();
        let processed_textures = original_model.processed_textures.clone();

        // Calculate grid dimensions for a square-ish layout
        let grid_size = (count as f32).sqrt().ceil() as usize;

        // Now create new models with the cloned data
        for i in 0..count {
            let row = i / grid_size;
            let col = i % grid_size;

            // Calculate offset from center
            let x_offset = (col as f32 - (grid_size as f32 / 2.0)) * grid_spacing;
            let z_offset = (row as f32 - (grid_size as f32 / 2.0)) * grid_spacing;

            // Create new vertices with offset
            let mut new_vertices = processed_vertices.clone();
            for vertex in &mut new_vertices {
                vertex.position[0] += x_offset;
                vertex.position[2] += z_offset;
            }

            // Create new model with the offset vertices
            let new_model = Model {
                meshes: Vec::new(), // Empty as we're using processed data
                materials: Vec::new(),
                processed_vertices: new_vertices,
                processed_indices: processed_indices.clone(),
                processed_materials: processed_materials.clone(),
                processed_textures: processed_textures.clone(),
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

    /// Adds a texture to a model at the specified index
    pub fn add_texture_to_model(
        &mut self,
        model_index: usize,
        texture_data: Vec<u32>,
        width: u32,
        height: u32,
    ) -> usize {
        if let Some(model) = self.models.get_mut(model_index) {
            // Create a new material info for the texture
            let texture_offset = model.processed_textures.len() as u32;

            // Add texture data
            model.processed_textures.extend_from_slice(&texture_data);

            // Create and add material info
            let texture_info = TextureInfo {
                offset: texture_offset,
                width,
                height,
                _padding: 0,
            };

            let material_info = MaterialInfo {
                texture_info,
                ambient: [0.1, 0.1, 0.1],
                _padding1: 0.0,
                specular: [0.5, 0.5, 0.5],
                _padding2: 0.0,
                diffuse: [1.0, 1.0, 1.0],
                shininess: 32.0,
                dissolve: 1.0,
                optical_density: 1.0,
                _padding3: [0.0, 0.0],
            };

            model.processed_materials.push(material_info);

            // Return the index of the new material
            model.processed_materials.len() - 1
        } else {
            panic!("Model index out of bounds");
        }
    }
}

pub struct SceneConfig {
    pub name: String,
    pub model_path: String,
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
    pub mode: crate::camera::CameraMode,
    pub position: [f32; 3],
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            distance: 2.0,
            theta: 0.0,
            phi: 0.0,
            target: [0.0, 0.0, 0.0],
            mode: crate::camera::CameraMode::Orbit,
            position: [0.0, 2.0, 5.0],
        }
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            model_path: String::new(),
            lights: Vec::new(),
            effects: None,
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        }
    }
}
