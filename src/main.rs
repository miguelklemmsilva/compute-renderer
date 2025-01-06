use std::time::Duration;

use effect::Effect;
use scene::{CameraConfig, SceneConfig, StressTestConfig};
use util::get_asset_path;

mod camera;
mod effect;
mod model;
mod performance;
mod scene;
mod util;
mod window;
mod gpu;

fn main() {
    let height = 900;
    let width = 1600;

    let lights = vec![
        ([0.0, 10.0, 5.0], [1.0, 1.0, 1.0], 1.0),
        ([-10.0, 0.0, 5.0], [1.0, 1.0, 1.0], 1.0),
        ([10.0, 0.0, 5.0], [1.0, 1.0, 1.0], 1.0),
    ];

    // List of scenes to benchmark
    let scenes = vec![
        SceneConfig {
            name: "Suzanne - Wave Effect".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::wave_horizontal(0.5, 3.0, 1.0)]),
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "African head - Dissolve Effect".to_string(),
            model_path: get_asset_path("african_head.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: Some(
                get_asset_path("african_head_diffuse.tga")
                    .to_string_lossy()
                    .to_string(),
            ),
            lights: lights.clone(),
            effects: Some(vec![Effect::dissolve(20.0, 1.0, 3.0)]),
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Suzanne - Smooth to Flat".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::smooth_to_flat(1.0, 4.0)]),
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "African head - Pixelate".to_string(),
            model_path: get_asset_path("african_head.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: Some(
                get_asset_path("african_head_diffuse.tga")
                    .to_string_lossy()
                    .to_string(),
            ),
            lights: lights.clone(),
            effects: Some(vec![Effect::pixelate(4.0, 32.0, 2.0)]),
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Suzanne - Voxelize".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::voxelize(15.0, 40.0, 1.)]),
            stress_test: None,
            camera_config: CameraConfig::default(),
            benchmark_duration_secs: 10,
        },
        // Stress test scenes with increasing model counts
        SceneConfig {
            name: "Stress Test - 10 Models".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            stress_test: Some(StressTestConfig {
                model_count: 10,
                grid_spacing: 3.0,
            }),
            camera_config: CameraConfig {
                distance: 3.0 * (10_f32).sqrt(),
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
            },
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Stress Test - 100 Models".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            stress_test: Some(StressTestConfig {
                model_count: 100,
                grid_spacing: 3.0,
            }),
            camera_config: CameraConfig {
                distance: 3.0 * (100_f32).sqrt(),
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
            },
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Stress Test - 1000 Models".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            stress_test: Some(StressTestConfig {
                model_count: 1000,
                grid_spacing: 3.0,
            }),
            camera_config: CameraConfig {
                distance: 3.0 * (1000_f32).sqrt(),
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
            },
            benchmark_duration_secs: 10,
        },
    ];

    for (i, scene_config) in scenes.iter().enumerate() {
        println!("Benchmarking scene {}: {}", i + 1, scene_config.name);
        let mut scene = crate::scene::Scene::new();
        let base_model = scene.add_model(&scene_config.model_path);

        // Handle stress test if configured
        if let Some(stress_config) = &scene_config.stress_test {
            if stress_config.model_count > 1 {
                scene.duplicate_model_for_stress_test(
                    base_model,
                    stress_config.model_count - 1,
                    stress_config.grid_spacing,
                );
            }
        }

        // Add texture if specified
        if let Some(texture_path) = &scene_config.texture_path {
            scene.add_texture_to_model(base_model, texture_path);
        }

        // Add lights from config
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
        scene.add_camera(crate::camera::Camera::new(
            scene_config.camera_config.distance,
            scene_config.camera_config.theta,
            scene_config.camera_config.phi,
            glam::Vec3::from(scene_config.camera_config.target),
            (width as f32) / (height as f32),
        ));
        scene.set_active_camera(0);

        // Create window and run benchmark
        let mut window = window::Window::new(width, height, scene);
        let performance_data = performance::benchmark_scene_with_duration(
            &mut window,
            Duration::from_secs(scene_config.benchmark_duration_secs),
        );

        performance::print_performance_data(&scene_config.name, i, &performance_data);
    }
}
