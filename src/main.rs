use std::time::Duration;

use camera::CameraMode;
use effect::Effect;
use performance::PerformanceCollector;
use scene::{CameraConfig, SceneConfig, StressTestConfig};
use util::get_asset_path;
use window::Window;

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
            model_path: get_asset_path("bmw.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            stress_test: None,
            camera_config: CameraConfig {
                mode: CameraMode::FirstPerson,
                ..Default::default()
            },
            benchmark_duration_secs: u64::MAX, // Run indefinitely until ESC
        },
        SceneConfig {
            name: "Suzanne - Wave Effect".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            },
            benchmark_duration_secs: 10,
        },
    ];

    // Create a single event loop and window for all scenes
    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");
    let window = winit::window::WindowBuilder::new()
        .with_title("Minimal Renderer - ESC to exit")
        .with_inner_size(winit::dpi::LogicalSize::new(width as f64, height as f64))
        .with_resizable(true)
        .build(&event_loop)
        .expect("Failed to create window");

    let scene = &scenes[0];

    let collector: PerformanceCollector = PerformanceCollector::new(
        scene.name.clone(),
        0,
        Duration::from_secs(scene.benchmark_duration_secs),
    );

    // Create the first scene
    let window = match create_scene_window(scene, width, height, &window, collector) {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Failed to create scene {}: {}", scene.name, e);
            return;
        }
    };

    window.run_with_event_loop(event_loop);
}

fn create_scene_window(
    scene_config: &SceneConfig,
    width: usize,
    height: usize,
    winit_window: &winit::window::Window,
    collector: PerformanceCollector,
) -> Result<Window, Box<dyn std::error::Error>> {
    let mut scene = scene::Scene::new();

    // Setup scene
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

    Window::new_with_window(width, height, scene, winit_window, collector)
}
