use std::time::Instant;

use window::Window;

mod camera;
mod clear_pass;
mod gpu;
mod model;
mod raster_pass;
mod scene;
mod util;
mod window;

fn main() {
    let height = 900;
    let width = 1000;

    let mut scene = scene::Scene::new();
    let model_index = {
        scene.add_model("assets/african_head.obj");
        scene.models.len() - 1
    };

    // Load the texture and get its index
    let texture_index = scene.add_texture("assets/african_head_diffuse.tga");

    // Apply the texture to the model
    if let Some(model) = scene.models.get_mut(model_index) {
        model.apply_texture(texture_index);
    }

    // Add camera and set active
    scene.add_camera(camera::Camera::new(
        2.0,
        0.0,
        3.0,
        glam::Vec3::ZERO,
        (width as f32) / (height as f32),
    ));
    scene.set_active_camera(0);

    let mut window = Window::new(width, height, scene);

    let mut last_time = Instant::now();
    let mut frame_start_time = Instant::now();
    let mut frame_count = 0;

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        let current_time = Instant::now();
        let delta_time = current_time.duration_since(frame_start_time).as_secs_f32();
        frame_start_time = current_time;

        if let Some(camera) = window.scene.get_active_camera_mut() {
            camera.update_over_time(delta_time);
        }

        // Render and display the frame
        pollster::block_on(window.update());

        // FPS counting
        frame_count += 1;
        if current_time.duration_since(last_time).as_secs_f32() >= 1.0 {
            println!("FPS: {}", frame_count);
            frame_count = 0;
            last_time = current_time;
        }
    }
}
