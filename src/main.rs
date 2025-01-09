use std::time::Duration;

use camera::CameraMode;
use effect::{Effect, WaveDirection};
use scene::{CameraConfig, SceneConfig, StressTestConfig};
use util::get_asset_path;

mod camera;
mod effect;
mod gpu;
mod model;
mod performance;
mod scene;
mod util;
mod window;

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
        // Interactive Scene
        SceneConfig {
            name: "Interactive Scene".to_string(),
            model_path: get_asset_path("room obj.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            stress_test: None,
            camera_config: CameraConfig {
                mode: CameraMode::FirstPerson,
                distance: 0.0,
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
                position: [0.0, 0.0, 0.0],
            },
            benchmark_duration_secs: u64::MAX, // Run indefinitely until ESC
        },
        SceneConfig {
            name: "Suzanne - Wave Effect".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::wave(0.5, 1.0, 1.0, WaveDirection::Horizontal)]),
            stress_test: None,
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                ..Default::default()
            },
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Suzanne - Edge Melt Effect".to_string(),
            model_path: get_asset_path("african_head.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: Some(
                get_asset_path("african_head_diffuse.tga")
                    .to_string_lossy()
                    .to_string(),
            ),
            lights: lights.clone(),
            effects: Some(vec![Effect::edge_melt(0.33, 1.0)]),
            stress_test: None,
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                distance: 2.0,
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
                position: [0.0, 2.0, 5.0],
            },
            benchmark_duration_secs: 100,
        },
        SceneConfig {
            name: "Suzanne - Voxelize".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::voxelize(0.5, 5.0)]),
            stress_test: None,
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                distance: 2.0,
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
                position: [0.0, 2.0, 5.0],
            },
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
                mode: CameraMode::Orbit,
                distance: 3.0 * (10_f32).sqrt(),
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
                position: [0.0, 2.0, 5.0],
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
                mode: CameraMode::Orbit,
                distance: 3.0 * (100_f32).sqrt(),
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
                position: [0.0, 2.0, 5.0],
            },
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Stress Test - 1000 Models".to_string(),
            model_path: get_asset_path("african_head.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            stress_test: Some(StressTestConfig {
                model_count: 1000,
                grid_spacing: 3.0,
            }),
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                distance: 3.0 * (1000_f32).sqrt(),
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
                position: [0.0, 2.0, 5.0],
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
        let camera = match scene_config.camera_config.mode {
            camera::CameraMode::FirstPerson => camera::Camera::new_first_person(
                glam::Vec3::from(scene_config.camera_config.position),
                (width as f32) / (height as f32),
            ),
            camera::CameraMode::Orbit => camera::Camera::new(
                scene_config.camera_config.distance,
                scene_config.camera_config.theta,
                scene_config.camera_config.phi,
                glam::Vec3::from(scene_config.camera_config.target),
                (width as f32) / (height as f32),
            ),
        };
        scene.add_camera(camera);
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
