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
    #[arg(long, default_value = "false", help = "Run benchmark scenes")]
    benchmarks: bool,

    /// Window width in pixels
    #[arg(long, default_value_t = 1024, help = "Window width in pixels")]
    width: u32,

    /// Window height in pixels
    #[arg(long, default_value_t = 768, help = "Window height in pixels")]
    height: u32,

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

    let scenes = if cli.benchmarks {
        let sportscar_scene = SceneConfig {
            model_path: "sportsCar/sportsCar.obj".to_string(),
            camera_config: CameraConfig {
                position: [1.1757767, 0.4654234, 3.2008126],
                mode: camera::CameraMode::FirstPerson,
                ..Default::default()
            },
            benchmark_duration_secs: 5,
            ..Default::default()
        };

        let suzanne_scene = SceneConfig {
            model_path: "suzanne.obj".to_string(),
            camera_config: CameraConfig {
                distance: 2.0,
                position: [0.0, 0.0, 3.0],
                mode: camera::CameraMode::FirstPerson,
                ..Default::default()
            },
            benchmark_duration_secs: 5,
            ..Default::default()
        };

        let exterior_scene = SceneConfig {
            model_path: "exterior/Exterior.obj".to_string(),
            camera_config: CameraConfig {
                position: [-525.80194, 168.52838, 260.81876],
                mode: camera::CameraMode::FirstPerson,
                ..Default::default()
            },
            benchmark_duration_secs: 5,
            ..Default::default()
        };

        let san_miguel_scene = SceneConfig {
            model_path: "San_Miguel/san-miguel-low-poly.obj".to_string(),
            camera_config: CameraConfig {
                position: [13.566635, 2.6288567, 10.243919],
                target: [13.587516, 2.5521376, 9.247086],
                mode: camera::CameraMode::FirstPerson,
                ..Default::default()
            },
            benchmark_duration_secs: 5,
            ..Default::default()
        };

        vec![
            san_miguel_scene.clone(),
            SceneConfig {
                backend_type: BackendType::WgpuPipeline,
                ..san_miguel_scene
            },
            exterior_scene.clone(),
            SceneConfig {
                backend_type: BackendType::WgpuPipeline,
                ..exterior_scene
            },
            suzanne_scene.clone(),
            SceneConfig {
                backend_type: BackendType::WgpuPipeline,
                ..suzanne_scene
            },
            sportscar_scene.clone(),
            SceneConfig {
                backend_type: BackendType::WgpuPipeline,
                ..sportscar_scene
            },
        ]
    } else {
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
        vec![SceneConfig {
            model_path: cli.model_path,
            camera_config,
            backend_type,
            benchmark_duration_secs: cli.benchmark_duration_secs,
            // effect: Some(Effect::mirage(3.0, 0.2, 1.0)),
            ..Default::default()
        }]
    };

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
            eprintln!(
                "Failed to create scene {}: {}",
                scene_config.scene_name(),
                e
            );
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
