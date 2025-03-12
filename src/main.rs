use clap::{Parser, Subcommand};
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

    /// Path to the 3D model (OBJ)
    #[arg(long, default_value = "suzanne.obj", help = "Path to the .obj file")]
    model_path: String,

    /// Camera mode: 'first-person' or 'orbit'
    #[arg(long, default_value = "first-person", help = "Camera mode")]
    camera_mode: String,

    /// Backend type: 'wgpu' or 'custom'
    #[arg(long, default_value = "custom", help = "Render backend type")]
    backend_type: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run benchmarks with a specified starting offset (zero-indexed)
    Benchmarks {
        /// Offset to start benchmarks (zero-indexed)
        #[arg(long, default_value_t = 0, help = "Offset to start benchmarks (zero-indexed)")]
        offset: usize,
    },
}

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    let width = cli.width as usize;
    let height = cli.height as usize;

    // Select scenes and determine starting scene based on whether the benchmarks subcommand was used.
    let (scenes, start_offset) = match &cli.command {
        Some(Commands::Benchmarks { offset }) => {
            let benchmark_duration_secs = 5;
            let vokselia_spawn_scene = SceneConfig {
                model_path: "vokselia_spawn/vokselia_spawn.obj".to_string(),
                camera_config: CameraConfig {
                    position: [0.0, 0.86896104, 1.4793645],
                    mode: camera::CameraMode::FirstPerson,
                    ..Default::default()
                },
                benchmark_duration_secs,
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
                benchmark_duration_secs,
                ..Default::default()
            };

            let exterior_scene = SceneConfig {
                model_path: "exterior/Exterior.obj".to_string(),
                camera_config: CameraConfig {
                    position: [-525.80194, 168.52838, 260.81876],
                    mode: camera::CameraMode::FirstPerson,
                    ..Default::default()
                },
                benchmark_duration_secs,
                ..Default::default()
            };

            let san_miguel_scene = SceneConfig {
                model_path: "San_Miguel/san-miguel-low-poly.obj".to_string(),
                camera_config: CameraConfig {
                    position: [13.566635, 2.6288567, 10.243919],
                    mode: camera::CameraMode::FirstPerson,
                    ..Default::default()
                },
                benchmark_duration_secs,
                ..Default::default()
            };

            let scenes = vec![
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
                vokselia_spawn_scene.clone(),
                SceneConfig {
                    backend_type: BackendType::WgpuPipeline,
                    ..vokselia_spawn_scene
                },
            ];

            if *offset >= scenes.len() {
                eprintln!(
                    "Invalid offset: {}. There are only {} scenes available for benchmarks.",
                    offset,
                    scenes.len()
                );
                std::process::exit(1);
            }

            (scenes, *offset)
        }
        _ => {
            // Regular mode: create a single scene based on CLI parameters.
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

            let backend_type = match cli.backend_type.as_str() {
                "wgpu" => BackendType::WgpuPipeline,
                "custom" => BackendType::CustomPipeline,
                other => {
                    eprintln!(
                        "Invalid backend type '{}'. Use 'wgpu' or 'custom'.",
                        other
                    );
                    std::process::exit(1);
                }
            };

            let scene_config = SceneConfig {
                model_path: cli.model_path,
                camera_config,
                backend_type,
                // effect: Some(effect::Effect::voxelize(3.0, 0.2)),
                ..Default::default()
            };

            (vec![scene_config], 0)
        }
    };

    // Create a single event loop for all scenes.
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Select the starting scene using the provided offset (or 0 for normal mode).
    let scene_config = &scenes[start_offset];

    let scene = pollster::block_on(scene::Scene::from_config(scene_config, width, height));

    // Create window with the same backend type as the scene.
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

    // Store scenes in window for cycling.
    window.set_scene_configs(scenes);

    // Run the event loop with our application handler.
    event_loop
        .run_app(&mut window)
        .expect("Failed to run application");
}