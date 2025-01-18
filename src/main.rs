use std::time::Duration;

use camera::CameraMode;
use effect::Effect;
use egui_winit::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use performance::PerformanceCollector;
use scene::{CameraConfig, SceneConfig, StressTestConfig};
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

    let lights = vec![([0.0, -100.0, 0.0], [1.0, 1.0, 1.0], 10000.0)];

    // List of scenes to benchmark
    let scenes = vec![
        // Interactive Scene
        SceneConfig {
            name: "Interactive Scene".to_string(),
            model_path: String::from("bmw/bmw.obj"),
            lights: lights.clone(),
            effects: None,
            stress_test: None,
            camera_config: CameraConfig {
                mode: CameraMode::FirstPerson,
                position: [0.0, 0.0, 0.0],
                distance: 0.0,
                theta: 0.0,
                phi: 0.0,
                target: [0.0, 0.0, 0.0],
            },
            benchmark_duration_secs: u64::MAX, // Run indefinitely until ESC
        },
        SceneConfig {
            name: "Suzanne - Wave Effect".to_string(),
            model_path: String::from("suzanne.obj"),
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
            model_path: String::from("african_head.obj"),
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
            model_path: String::from("suzanne.obj"),
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
    ];

    // Create a single event loop for all scenes
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let scene_config = &scenes[0];

    let collector: PerformanceCollector = PerformanceCollector::new(
        scene_config.name.clone(),
        0,
        Duration::from_secs(scene_config.benchmark_duration_secs),
    );

    // Create the scene from config
    let mut scene = scene::Scene::new();

    // Setup scene
    pollster::block_on(async {
        let base_model = scene.add_obj_with_mtl(&scene_config.model_path).await;

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
    });

    // Create the first scene
    let mut window = match Window::new_with_window(width, height, scene, collector) {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Failed to create scene {}: {}", scene_config.name, e);
            return;
        }
    };

    // Run the event loop with our application handler
    event_loop
        .run_app(&mut window)
        .expect("Failed to run application");
}
