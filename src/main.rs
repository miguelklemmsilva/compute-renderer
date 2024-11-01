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
    let mut scene = scene::Scene::new();
    scene.add_model(model::Model::new(
        "assets/ANH_SABER.obj",
        model::FileType::Obj,
    ));
    let mut window = Window::new(800, 600, scene);

    let mut last_time = Instant::now();
    let mut frame_count = 0;

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        pollster::block_on(window.update());

        frame_count += 1;
        let current_time = Instant::now();
        if current_time.duration_since(last_time).as_secs_f32() >= 1.0 {
            println!("FPS: {}", frame_count);
            frame_count = 0;
            last_time = current_time;
        }
    }
}
