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
    long_about = "A command-line tool for rendering 3D scenes using different backends and effects. \
                 Supports custom software rasterization and WGPU pipeline rendering. \
                 Allows selecting camera modes, effects, and benchmarking scenes."
)]
struct Cli {
    /// Window width in pixels (default: 1024)
    #[arg(long, default_value_t = 1024, help = "Set the width of the application window (in pixels)")]
    width: u32,

    /// Window height in pixels (default: 768)
    #[arg(long, default_value_t = 768, help = "Set the height of the application window (in pixels)")]
    height: u32,

    /// Path to the 3D model (OBJ format, default: suzanne.obj)
    #[arg(long, default_value = "suzanne.obj", help = "Specify the path to a 3D model file in .obj format")]
    model_path: String,

    /// Camera mode selection (default: first-person)
    /// Options:
    /// - first-person: Controls behave like an FPS game (WASD + mouse)
    /// - orbit: Rotates around the object with mouse drag
    #[arg(long, default_value = "first-person", help = "Choose camera mode: 'first-person' or 'orbit'")]
    camera_mode: String,

    /// Rendering backend selection (default: custom)
    /// Options:
    /// - custom: Software rasterization using compute shaders
    /// - wgpu: Hardware-accelerated rendering via WGPU
    #[arg(long, default_value = "custom", help = "Select rendering backend: 'wgpu' or 'custom'")]
    backend_type: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run performance benchmarks across different scenes
    ///
    /// Available scenes:
    /// 0 - San Miguel (custom pipeline)
    /// 1 - San Miguel (WGPU pipeline)
    /// 2 - Exterior (custom pipeline)
    /// 3 - Exterior (WGPU pipeline)
    /// 4 - Suzanne (custom pipeline)
    /// 5 - Suzanne (WGPU pipeline)
    /// 6 - Vokselia Spawn (custom pipeline)
    /// 7 - Vokselia Spawn (WGPU pipeline)
    Benchmarks {
        /// Offset to start benchmarks (valid values: 0-7)
        #[arg(long, default_value_t = 0, help = "Scene index to start benchmarks from (0-7)")]
        offset: usize,
    },
    /// Apply a visual effect to the scene
    ///
    /// Available effects:
    /// - voxelize: Converts the scene into a voxelized form
    /// - edge_melt: Warps edges dynamically
    /// - mirage: Distorts the scene with wave-like motion
    /// - wave: Oscillates scene geometry in different patterns
    /// - none: Disables effects
    Effect {
        /// Effect type (default: voxelize)
        #[arg(long, default_value = "voxelize", help = "Choose effect: 'voxelize', 'edge_melt', 'mirage', 'wave', or 'none'")]
        effect: String,
        /// Parameter 1: Controls voxel size, amplitude, or intensity (default: 3.0)
        #[arg(long, default_value_t = 3.0, help = "Primary effect parameter (varies by effect type)")]
        param1: f32,
        /// Parameter 2: Controls speed or frequency (default: 0.2)
        #[arg(long, default_value_t = 0.2, help = "Secondary effect parameter (varies by effect type)")]
        param2: f32,
        /// Parameter 3: For wave & mirage effects (default: 1.0)
        #[arg(long, default_value_t = 1.0, help = "Third effect parameter (for wave & mirage effects)")]
        param3: f32,
        /// Parameter 4: Wave effect direction (0 = Vertical, 1 = Horizontal, 2 = Radial) (default: 0)
        #[arg(long, default_value_t = 0, help = "Wave direction (0=Vertical, 1=Horizontal, 2=Radial)")]
        param4: u32,
    },
}


fn main() {    
    let cli = Cli::parse();

    let width = cli.width as usize;
    let height = cli.height as usize;

    // Determine if a visual effect is requested and configure it accordingly. This enables dynamic scene modifications.
    let effect = match &cli.command {
        Some(Commands::Effect {
            effect,
            param1,
            param2,
            param3,
            param4,
        }) => match effect.as_str() {
            "voxelize" => Some(effect::Effect::voxelize(*param1, *param2)),
            "edge_melt" => Some(effect::Effect::edge_melt(*param1, *param2)),
            "mirage" => Some(effect::Effect::mirage(*param1, *param2, *param3)),
            "wave" => {
                // For the 'wave' effect, determine its direction based on user input to ensure the correct visual transformation.
                let direction = match param4 {
                    0 => effect::WaveDirection::Vertical,
                    1 => effect::WaveDirection::Horizontal,
                    2 => effect::WaveDirection::Radial,
                    _ => {
                        eprintln!(
                                "Invalid wave direction {}. Use 0 for Vertical, 1 for Horizontal, or 2 for Radial.",
                                param4
                            );
                        std::process::exit(1);
                    }
                };
                Some(effect::Effect::wave(*param1, *param2, *param3, direction))
            }
            "none" => None,
            other => {
                eprintln!(
                        "Invalid effect '{}'. Use 'voxelize', 'edge_melt', 'mirage', 'wave', or 'none'.",
                        other
                    );
                std::process::exit(1);
            }
        },
        _ => None,
    };

    // Decide between benchmark mode and regular mode. Benchmark mode evaluates performance over predefined scenes.
    let (scenes, start_offset) = match cli.command {
        Some(Commands::Benchmarks { offset }) => {
            // Set benchmark duration and initialize various scene configurations to test both custom and WGPU pipelines.
            let benchmark_duration_secs = 30;
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

            if offset >= scenes.len() {
                eprintln!(
                    "Invalid offset: {}. There are only {} scenes available for benchmarks.",
                    offset,
                    scenes.len()
                );
                std::process::exit(1);
            }

            (scenes, offset)
        }
        _ => {
            // Regular mode: Build a scene using user-specified camera mode, backend, and model path.
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
                    eprintln!("Invalid backend type '{}'. Use 'wgpu' or 'custom'.", other);
                    std::process::exit(1);
                }
            };

            let scene_config = SceneConfig {
                model_path: cli.model_path,
                camera_config: CameraConfig {
                    position: [13.566635, 2.6288567, 10.243919],
                    ..camera_config
                },
                backend_type,
                effect,
                ..Default::default()
            };

            (vec![scene_config], 0)
        }
    };

    // Create a centralized event loop for rendering and event handling, crucial for a responsive application.
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Select the starting scene based on the provided offset (or default to the first scene).
    let scene_config = &scenes[start_offset];

    // Asynchronously initialize the scene to handle setup operations such as resource loading.
    let scene = pollster::block_on(scene::Scene::from_config(scene_config, width, height));

    // Create the rendering window with a backend matching the scene configuration to ensure compatibility.
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

    // Store all scene configurations in the window to enable switching between scenes during runtime.
    window.set_scene_configs(scenes);

    // Start the event loop which continuously renders the scene and processes user input.
    event_loop
        .run_app(&mut window)
        .expect("Failed to run application");
}
