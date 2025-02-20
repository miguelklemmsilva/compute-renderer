use crate::model::Model;
use crate::window::BackendType;
use crate::{camera, effect::Effect, custom_pipeline};
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

        scene
            .add_obj_with_mtl(&scene_config.model_path, scene_config.backend_type)
            .await;

        // Add lights from config if not a stress test
        for (position, color, intensity) in &scene_config.lights {
            scene.add_light(*position, *color, *intensity);
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
    pub async fn add_obj_with_mtl(&mut self, obj_path: &str, backend_type: BackendType) -> usize {
        // (A) Load geometry + textures from the .obj + .mtl
        let model = Model::new(obj_path, backend_type).await;
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

    pub fn update(&mut self, gpu: &mut custom_pipeline::gpu::GPU, delta_time: Duration) {
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

    pub fn add_effect(&mut self, effect: Effect) -> usize {
        self.effects.push(effect);
        self.effects.len() - 1
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
    // Camera configuration
    pub camera_config: CameraConfig,
    // Benchmark duration in seconds
    pub benchmark_duration_secs: u64,
    pub backend_type: BackendType,
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
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
            backend_type: BackendType::WgpuPipeline,
        }
    }
}
