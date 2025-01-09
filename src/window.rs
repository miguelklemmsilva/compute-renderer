use minifb::{CursorStyle, Key, MouseMode};
use pollster::block_on;
use std::time::Duration;

use crate::{gpu, scene};

pub struct Window {
    window: minifb::Window,
    pub gpu: gpu::gpu::GPU,
    height: usize,
    width: usize,
    pub scene: scene::Scene,
    last_mouse_pos: Option<(f32, f32)>,
}

impl Window {
    pub fn new(width: usize, height: usize, scene: scene::Scene) -> Window {
        let gpu = block_on(gpu::gpu::GPU::new(width, height, &scene));
        let mut window = minifb::Window::new(
            "Minimal Renderer - ESC to exit",
            width,
            height,
            minifb::WindowOptions {
                resize: true,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

        Window {
            gpu,
            window,
            height,
            width,
            scene,
            last_mouse_pos: None,
        }
    }

    pub async fn update(&mut self, delta_time: Duration) {
        // Handle keyboard input
        let forward = self.window.is_key_down(Key::W);
        let backward = self.window.is_key_down(Key::S);
        let left = self.window.is_key_down(Key::A);
        let right = self.window.is_key_down(Key::D);
        let up = self.window.is_key_down(Key::Space);
        let down = self.window.is_key_down(Key::LeftShift);

        if let Some(camera) = self.scene.get_active_camera_mut() {
            camera.process_keyboard(
                forward,
                backward,
                left,
                right,
                up,
                down,
                delta_time.as_secs_f32(),
            );
        }

        // Handle mouse input
        if let Some((x, y)) = self.window.get_mouse_pos(MouseMode::Clamp) {
            let x = x as f32;
            let y = y as f32;

            if let Some((last_x, last_y)) = self.last_mouse_pos {
                let x_offset = x - last_x;
                let y_offset = last_y - y; // Reversed since y-coordinates range from bottom to top

                if self.window.get_mouse_down(minifb::MouseButton::Left) {
                    if let Some(camera) = self.scene.get_active_camera_mut() {
                        camera.process_mouse(x_offset, y_offset);
                    }
                }
            }

            self.last_mouse_pos = Some((x, y));
        }

        self.scene.update(&mut self.gpu, delta_time);

        let buffer = self
            .gpu
            .execute_pipeline(self.width, self.height, &self.scene)
            .await;

        // Update the window with the buffer
        self.window
            .update_with_buffer(&buffer, self.width, self.height)
            .expect("Failed to update window");
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.window.is_key_down(key)
    }
}
