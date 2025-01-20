use std::time::Duration;

use camera::CameraMode;
use effect::Effect;
use performance::PerformanceCollector;
use scene::{CameraConfig, SceneConfig};
use window::Window;
use winit::event_loop::{ControlFlow, EventLoop};

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

    let lights = vec![([0.0, -100.0, 0.0], [1.0, 1.0, 1.0], 100.0)];

    // List of scenes to benchmark
    let scenes = vec![
        SceneConfig {
            name: "Suzanne - Edge Melt Effect".to_string(),
            model_path: String::from("african_head.obj"),
            texture_path: Some(String::from("african_head_diffuse.tga")),
            lights: lights.clone(),
            effects: Some(vec![Effect::edge_melt(0.33, 1.0)]),
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                distance: 2.0,
                ..Default::default()
            },
            benchmark_duration_secs: 100,
        },
        // Interactive Scene
        SceneConfig {
            name: "Interactive Scene".to_string(),
            model_path: String::from("erato/erato.obj"),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
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
        // Stress Test Scene - Multiple Models
        SceneConfig {
            name: "Stress Test - 100 Models".to_string(),
            model_path: String::from("suzanne.obj"),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                distance: 30.0, // Increased distance to view the entire grid
                theta: std::f32::consts::PI / 4.0, // 45 degrees
                phi: std::f32::consts::PI / 4.0, // 45 degrees
                target: [0.0, 0.0, 0.0],
                ..Default::default()
            },
            benchmark_duration_secs: 30, // 30 seconds benchmark
        },
        SceneConfig {
            name: "Suzanne - Wave Effect".to_string(),
            model_path: String::from("suzanne.obj"),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            camera_config: CameraConfig {
                mode: CameraMode::Orbit,
                ..Default::default()
            },
            benchmark_duration_secs: 10,
        },
        SceneConfig {
            name: "Suzanne - Voxelize".to_string(),
            model_path: String::from("suzanne.obj"),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::voxelize(0.5, 5.0)]),
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

    // Create the first scene
    let current_scene = 0;
    let scene_config = &scenes[current_scene];

    let collector = PerformanceCollector::new(
        scene_config.name.clone(),
        current_scene,
        Duration::from_secs(scene_config.benchmark_duration_secs),
    );

    let scene = pollster::block_on(scene::Scene::from_config(scene_config, width, height));

    let mut window = match Window::new_with_window(width, height, scene, collector) {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Failed to create scene {}: {}", scene_config.name, e);
            return;
        }
    };

    // Store scenes in window for cycling
    window.set_scene_configs(scenes);

    // Run the event loop with our application handler
    event_loop
        .run_app(&mut window)
        .expect("Failed to run application");
}
