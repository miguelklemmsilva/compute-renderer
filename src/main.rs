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

    let lights = vec![([0.0, -100.0, 0.0], [1.0, 1.0, 1.0], 1000.0)];

    // List of scenes to benchmark
    let scenes = vec![
        SceneConfig {
            name: "test".to_string(),
            model_path: String::from("sportsCar/sportsCar.obj"),
            texture_path: None,
            lights: lights.clone(),
            effects: None,
            camera_config: CameraConfig {
                mode: CameraMode::FirstPerson,
                ..Default::default()
            },
            benchmark_duration_secs: u64::MAX,
        }
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
