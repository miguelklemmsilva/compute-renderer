use crate::camera::{Camera, CameraMode};
use crate::effect::Effect;
use crate::model::Model;
use crate::window::BackendType;
use crate::{camera, custom_pipeline};
use std::time::Duration;
use std::u64;

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
    pub effect: Option<Effect>,
    pub time: f32,
    pub total_tris: f32,
    pub gx_tris: u32,
    pub gy_tris: u32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            models: vec![],
            cameras: vec![],
            active_camera: None,
            lights: vec![],
            effect: None,
            time: 0.0,
            total_tris: 0.0,
            gx_tris: 0,
            gy_tris: 0,
        }
    }

    /// Creates a new scene from a scene configuration
    pub async fn from_config(scene_config: &SceneConfig, width: usize, height: usize) -> Scene {
        let mut scene = Scene::new();

        scene
            .add_obj_with_mtl(&scene_config.model_path, scene_config.backend_type)
            .await;

        for (position, color, intensity) in &scene_config.lights {
            scene.add_light(*position, *color, *intensity);
        }

        if let Some(effect) = &scene_config.effect {
            scene.effect = Some(effect.clone());
        }

        // Add camera and set active
        let camera = match scene_config.camera_config.mode {
            CameraMode::FirstPerson => Camera::new_first_person(
                glam::Vec3::from(scene_config.camera_config.position),
                width as f32 / height as f32,
            ),
            CameraMode::Orbit => Camera::new(
                scene_config.camera_config.distance,
                scene_config.camera_config.theta,
                scene_config.camera_config.phi,
                glam::Vec3::from(scene_config.camera_config.target),
                width as f32 / height as f32,
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
        let total_indices = model.processed_indices.len();

        // do these calculations here so that it does not need to be recalculated every frame
        self.total_tris = (total_indices / 3) as f32;

        self.gx_tris = self.total_tris.sqrt().ceil() as u32;
        self.gy_tris = (self.total_tris / (self.gx_tris as f32)).ceil() as u32;

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

    pub fn update_buffers(
        &mut self,
        renderer: &mut custom_pipeline::renderer::CustomRenderer,
        delta_time: Duration,
    ) {
        self.time += delta_time.as_secs_f32();

        if let Some(effect) = &mut self.effect {
            effect.update(delta_time);
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

            renderer.queue.write_buffer(
                &renderer.buffers.camera_buffer,
                0,
                bytemuck::bytes_of(&camera_uniform),
            );
        }

        // Update lights
        renderer.queue.write_buffer(
            &renderer.buffers.light_buffer,
            0,
            bytemuck::cast_slice(&self.lights),
        );

        // Update effects only if there are any
        if let Some(effect) = &self.effect {
            let mut effect_uniform = crate::effect::EffectUniform::default();
            effect_uniform.update(effect, self.time);
            renderer.queue.write_buffer(
                &renderer.buffers.effect_buffer,
                0,
                bytemuck::bytes_of(&effect_uniform),
            );
        } else {
            // Write a default "no effect" state
            let effect_uniform = crate::effect::EffectUniform::default();
            renderer.queue.write_buffer(
                &renderer.buffers.effect_buffer,
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
}

#[derive(Clone)]
pub struct SceneConfig {
    pub model_path: String,
    pub lights: Vec<(
        /* position */ [f32; 3],
        /* color */ [f32; 3],
        /* intensity */ f32,
    )>,
    pub effect: Option<Effect>,
    // Camera configuration
    pub camera_config: CameraConfig,
    // Benchmark duration in seconds
    pub benchmark_duration_secs: u64,
    pub backend_type: BackendType,
}

impl SceneConfig {
    pub fn scene_name(&self) -> String {
        format!("Scene {} - {} Pipeline", self.model_path, self.backend_type)
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            model_path: "suzanne.obj".to_string(),
            lights: vec![
                ([0.0, 0.0, 0.0], [1.0, 0.9, 0.8], 1.0),
                // Fill light
                ([-5.0, 3.0, 0.0], [0.3, 0.4, 0.5], 0.5),
            ],
            effect: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: u64::MAX,
            backend_type: BackendType::CustomPipeline,
        }
    }
}

#[derive(Clone)]
pub struct CameraConfig {
    pub distance: f32,
    pub theta: f32,
    pub phi: f32,
    pub target: [f32; 3],
    pub mode: crate::camera::CameraMode,
    pub position: [f32; 3],
}

impl CameraConfig {
    #[allow(dead_code)]
    pub fn new_first_person() -> Self {
        Self {
            distance: 0.0,
            mode: crate::camera::CameraMode::FirstPerson,
            position: [0.0, 0.0, 0.0],
            ..Default::default()
        }
    }
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            distance: 4.0,
            theta: 0.0,
            phi: 0.0,
            target: [0.0, 0.0, 0.0],
            mode: crate::camera::CameraMode::Orbit,
            position: [0.0, 2.0, 5.0],
        }
    }
}
