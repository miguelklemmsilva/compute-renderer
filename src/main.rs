use std::time::Duration;

use effect::Effect;
use scene::SceneConfig;
use util::get_asset_path;

mod camera;
mod clear_pass;
mod effect;
mod gpu;
mod model;
mod performance;
mod raster_pass;
mod scene;
mod util;
mod window;

fn main() {
    let height = 900;
    let width = 1600;

    let lights = vec![
        ([0.0, 10.0, 5.0], [1.0, 1.0, 1.0], 1.0), // Yellow top light
        ([-10.0, 0.0, 5.0], [1.0, 1.0, 1.0], 1.0), // Cyan left light
        ([10.0, 0.0, 5.0], [1.0, 1.0, 1.0], 1.0), // Magenta right light
    ];

    // List of scenes to benchmark
    let scenes = vec![
        SceneConfig {
            name: "Suzanne - Wave Effect".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::wave_horizontal(
                0.5,
                3.0,
                1.0
            )]),
        },
        SceneConfig {
            name: "African head - Dissolve Effect".to_string(),
            model_path: get_asset_path("african_head.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: Some(
                get_asset_path("african_head_diffuse.tga")
                    .to_string_lossy()
                    .to_string(),
            ),
            lights: lights.clone(),
            effects: Some(vec![Effect::dissolve(20.0, 1.0, 3.0)]),
        },
        SceneConfig {
            name: "Suzanne - Smooth to Flat".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::smooth_to_flat(1.0, 4.0)]),
        },
        SceneConfig {
            name: "African head - Pixelate".to_string(),
            model_path: get_asset_path("african_head.obj")
                .to_string_lossy()
                .to_string(),
            texture_path: Some(
                get_asset_path("african_head_diffuse.tga")
                    .to_string_lossy()
                    .to_string(),
            ),
            lights: lights.clone(),
            effects: Some(vec![Effect::pixelate(4.0, 32.0, 2.0)]),
        },
        SceneConfig {
            name: "Suzanne - Voxelize".to_string(),
            model_path: get_asset_path("suzanne.obj").to_string_lossy().to_string(),
            texture_path: None,
            lights: lights.clone(),
            effects: Some(vec![Effect::voxelize(15.0, 40.0, 1.)]),
        },
    ];

    for (i, scene_config) in scenes.iter().enumerate() {
        println!("Benchmarking scene {}: {}", i + 1, scene_config.name);

        let performance_data: performance::PerformanceData =
            performance::benchmark_scene(scene_config, width, height);
        performance::print_performance_data(&scene_config.name, i, &performance_data);
    }
}
