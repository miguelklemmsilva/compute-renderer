use std::time::{Duration, Instant};
use sysinfo::{get_current_pid, System};

use crate::{scene::SceneConfig, window::Window};

pub struct PerformanceData {
    pub avg_fps: f64,
    pub min_fps: f64,
    pub max_fps: f64,
    pub fps_5_percent_low: f64,
    pub fps_1_percent_low: f64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

/// Benchmarks a scene using a pre-configured Window instance
pub fn benchmark_window(window: &mut Window) -> PerformanceData {
    // Initialize the system information once
    let mut system = System::new_all();
    let current_pid = get_current_pid().expect("Failed to get current PID");

    // Initialize performance data
    let mut frame_times = Vec::new();
    let mut cpu_usages = Vec::new();
    let mut memory_usages = Vec::new();

    let benchmark_duration = Duration::from_secs(10); // Run each scene for 10 seconds
    let benchmark_start_time = Instant::now();

    let mut last_frame_time = Instant::now();

    while window.is_open()
        && !window.is_key_down(minifb::Key::Escape)
        && benchmark_start_time.elapsed() < benchmark_duration
    {
        let frame_start_time = Instant::now();

        // Calculate delta_time since last frame
        let delta_time = frame_start_time
            .duration_since(last_frame_time)
            .as_secs_f32();

        // Update the camera (automatic movement only in orbit mode)
        if let Some(camera) = window.scene.get_active_camera_mut() {
            if let crate::camera::CameraMode::Orbit = camera.mode {
                camera.update_over_time(delta_time);
            }
        }

        // Render and display the frame
        pollster::block_on(window.update(Duration::from_secs_f32(delta_time)));

        last_frame_time = frame_start_time; // Update last_frame_time for next delta_time calculation

        // Calculate frame time
        let frame_time = frame_start_time.elapsed().as_secs_f64();
        frame_times.push(frame_time);

        // Update the system information
        system.refresh_cpu_all(); // Refresh CPU information
        system.refresh_memory(); // Refresh current process information

        // Collect CPU usage
        let cpu_usage = system.global_cpu_usage();
        cpu_usages.push(cpu_usage);

        // Collect memory usage
        if let Some(process) = system.process(current_pid) {
            let memory_usage = process.memory(); // in bytes
            memory_usages.push(memory_usage);
        } else {
            println!("Warning: Process with PID {:?} not found", current_pid);
            memory_usages.push(0);
        }
    }

    calculate_performance_metrics(&frame_times, &cpu_usages, &memory_usages)
}

/// Benchmarks a scene using a pre-configured Window instance and custom duration
pub fn benchmark_scene_with_duration(window: &mut Window, duration: Duration) -> PerformanceData {
    // Initialize the system information once
    let mut system = System::new_all();
    let current_pid = get_current_pid().expect("Failed to get current PID");

    // Initialize performance data
    let mut frame_times = Vec::new();
    let mut cpu_usages = Vec::new();
    let mut memory_usages = Vec::new();

    let benchmark_start_time = Instant::now();
    let mut last_frame_time = Instant::now();

    while window.is_open()
        && !window.is_key_down(minifb::Key::Escape)
        && benchmark_start_time.elapsed() < duration
    {
        let frame_start_time = Instant::now();

        // Calculate delta_time since last frame
        let delta_time = frame_start_time
            .duration_since(last_frame_time)
            .as_secs_f32();

        // Update the camera (automatic movement)
        if let Some(camera) = window.scene.get_active_camera_mut() {
            // camera.update_over_time(delta_time);
        }

        // Render and display the frame
        pollster::block_on(window.update(Duration::from_secs_f32(delta_time)));

        last_frame_time = frame_start_time; // Update last_frame_time for next delta_time calculation

        // Calculate frame time
        let frame_time = frame_start_time.elapsed().as_secs_f64();
        frame_times.push(frame_time);

        // Update the system information
        system.refresh_cpu_all(); // Refresh CPU information
        system.refresh_memory(); // Refresh current process information

        // Collect CPU usage
        let cpu_usage = system.global_cpu_usage();
        cpu_usages.push(cpu_usage);

        // Collect memory usage
        if let Some(process) = system.process(current_pid) {
            let memory_usage = process.memory(); // in bytes
            memory_usages.push(memory_usage);
        } else {
            println!("Warning: Process with PID {:?} not found", current_pid);
            memory_usages.push(0);
        }
    }

    calculate_performance_metrics(&frame_times, &cpu_usages, &memory_usages)
}

/// Benchmarks a scene using a SceneConfig
pub fn benchmark_scene(scene_config: &SceneConfig, width: usize, height: usize) -> PerformanceData {
    // Create the scene
    let mut scene = crate::scene::Scene::new();
    let model_index = scene.add_model(&scene_config.model_path);

    if let Some(texture_path) = &scene_config.texture_path {
        scene.add_texture_to_model(model_index, texture_path);
    }

    // Add lights from config
    for (position, color, intensity) in &scene_config.lights {
        scene.add_light(*position, *color, *intensity);
    }

    // Add effects from config
    if let Some(effects) = &scene_config.effects {
        for effect in effects {
            scene.add_effect(effect.clone());
        }
    }

    // Add camera and set active
    scene.add_camera(crate::camera::Camera::new(
        scene_config.camera_config.distance,
        scene_config.camera_config.theta,
        scene_config.camera_config.phi,
        glam::Vec3::from(scene_config.camera_config.target),
        (width as f32) / (height as f32),
    ));
    scene.set_active_camera(0);

    // Create the window and run benchmark
    let mut window = Window::new(width, height, scene);
    benchmark_scene_with_duration(
        &mut window,
        Duration::from_secs(scene_config.benchmark_duration_secs),
    )
}

/// Helper function to calculate performance metrics from collected data
fn calculate_performance_metrics(
    frame_times: &[f64],
    cpu_usages: &[f32],
    memory_usages: &[u64],
) -> PerformanceData {
    let avg_frame_time = frame_times.iter().sum::<f64>() / frame_times.len() as f64;
    let avg_fps = 1.0 / avg_frame_time;

    let min_frame_time = *frame_times
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_frame_time = *frame_times
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let min_fps = 1.0 / max_frame_time;
    let max_fps = 1.0 / min_frame_time;

    let avg_cpu_usage = cpu_usages.iter().sum::<f32>() / cpu_usages.len() as f32;
    let avg_memory_usage = memory_usages.iter().sum::<u64>() / memory_usages.len() as u64;

    // Calculate 5% and 1% low FPS
    let mut sorted_frame_times = frame_times.to_vec();
    sorted_frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let total_frames = sorted_frame_times.len();

    let percentile_5_index = (total_frames as f64 * 0.05).ceil() as usize;
    let slowest_5_percent_frames = &sorted_frame_times[(total_frames - percentile_5_index)..];
    let avg_5_percent_low_frame_time =
        slowest_5_percent_frames.iter().sum::<f64>() / slowest_5_percent_frames.len() as f64;
    let fps_5_percent_low = 1.0 / avg_5_percent_low_frame_time;

    let percentile_1_index = (total_frames as f64 * 0.01).ceil() as usize;
    let slowest_1_percent_frames = &sorted_frame_times[(total_frames - percentile_1_index)..];
    let avg_1_percent_low_frame_time =
        slowest_1_percent_frames.iter().sum::<f64>() / slowest_1_percent_frames.len() as f64;
    let fps_1_percent_low = 1.0 / avg_1_percent_low_frame_time;

    PerformanceData {
        avg_fps,
        min_fps,
        max_fps,
        fps_5_percent_low,
        fps_1_percent_low,
        cpu_usage: avg_cpu_usage,
        memory_usage: avg_memory_usage,
    }
}

pub fn print_performance_data(scene_name: &str, scene_index: usize, data: &PerformanceData) {
    println!(
        "Performance Data for Scene {}: {}",
        scene_index + 1,
        scene_name
    );
    println!("Average FPS: {:.2}", data.avg_fps);
    println!("Min FPS: {:.2}", data.min_fps);
    println!("Max FPS: {:.2}", data.max_fps);
    println!("5% Low FPS: {:.2}", data.fps_5_percent_low);
    println!("1% Low FPS: {:.2}", data.fps_1_percent_low);
    println!("Average CPU Usage: {:.2}%", data.cpu_usage);
    println!(
        "Average Memory Usage: {:.2} MB",
        data.memory_usage as f64 / (1024.0 * 1024.0)
    );
    println!("----------------------------------------");
}
