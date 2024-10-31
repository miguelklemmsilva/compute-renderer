use window::Window;

mod window;
mod raster_pass;
mod util;
mod gpu;
mod camera;
mod clear_pass;
mod scene;
mod model;

fn main() {
    let mut scene = scene::Scene::new();
    scene.add_model(model::Model::new("assets/ANH_SABER.obj", model::FileType::Obj));
    let mut window = Window::new(800, 600, scene);

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        pollster::block_on(window.update());
    }
}
