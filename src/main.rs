use clap::Parser;
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

#[derive(Parser, Debug)]
#[command(
    name = "Compute Renderer",
    version,
    about = "Runs a 3D scene with configurable parameters.",
    long_about = None
)]
struct Cli {
    /// Window width in pixels
    #[arg(long, default_value_t = 1024, help = "Window width in pixels")]
    width: u32,

    /// Window height in pixels
    #[arg(long, default_value_t = 768, help = "Window height in pixels")]
    height: u32,

    /// Name of the scene
    #[arg(long, default_value = "test_scene", help = "Name of the scene")]
    scene_name: String,

    /// Path to the 3D model (OBJ)
    #[arg(long, default_value = "suzanne.obj", help = "Path to the .obj file")]
    model_path: String,

    /// Camera mode: 'first-person' or 'orbit'
    #[arg(long, default_value = "first-person", help = "Camera mode")]
    camera_mode: String,

    /// Backend type: 'wgpu' or 'custom'
    #[arg(long, default_value = "custom", help = "Render backend type")]
    backend_type: String,

    /// Benchmark duration in seconds (if needed for performance testing)
    #[arg(long, default_value_t = u64::MAX, help = "Benchmark duration in seconds")]
    benchmark_duration_secs: u64,
}

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    let width = cli.width as usize;
    let height = cli.height as usize;

    // Determine the camera configuration based on user input
    let camera_config = match cli.camera_mode.as_str() {
        "first-person" => CameraConfig::new_first_person(),
        "orbit" => CameraConfig::default(),
        other => {
            eprintln!(
                "Invalid camera mode '{}'. Use 'first-person' or 'orbit'.",
                other
            );
            std::process::exit(1);
        }
    };

    // Determine the backend type (WGPU or Custom)
    let backend_type = match cli.backend_type.as_str() {
        "wgpu" => BackendType::WgpuPipeline,
        "custom" => BackendType::CustomPipeline,
        other => {
            eprintln!("Invalid backend type '{}'. Use 'wgpu' or 'custom'.", other);
            std::process::exit(1);
        }
    };

    // Construct a single SceneConfig based on CLI parameters
    let scenes = vec![SceneConfig {
        name: cli.scene_name,
        model_path: cli.model_path,
        camera_config,
        backend_type,
        benchmark_duration_secs: cli.benchmark_duration_secs,
        // effect: Some(Effect::mirage(3.0, 0.2, 1.0)),
        ..Default::default()
    }];

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
