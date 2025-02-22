use scene::{CameraConfig, SceneConfig};
use window::{BackendType, Window};
use winit::event_loop::{ControlFlow, EventLoop};

mod camera;
mod custom_pipeline;
mod effect;
mod model;
mod performance;
mod scene;
mod util;
mod vertex;
mod wgpu_pipeline;
mod window;

fn main() {
    let height = 900;
    let width = 1600;

    // List of scenes to benchmark
    let scenes = vec![
        SceneConfig {
            name: "test".to_string(),
            model_path: String::from("Exterior/exterior.obj"),
            camera_config: CameraConfig::new_first_person(),
            ..Default::default()
        },
        SceneConfig {
            name: "test".to_string(),
            model_path: String::from("suzanne.obj"),
            camera_config: CameraConfig::new_first_person(),
            backend_type: BackendType::WgpuPipeline,
            ..Default::default()
        },
    ];

    // Create a single event loop for all scenes
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create the first scene
    let current_scene = 0;
    let scene_config = &scenes[current_scene];

    let scene = pollster::block_on(scene::Scene::from_config(scene_config, width, height));

    // Create window with the same backend type as the scene
    let mut window = match Window::new_with_window(width, height, scene, scene_config.backend_type)
    {
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
