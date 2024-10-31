use pollster::block_on;

use crate::{gpu, scene};

pub struct Window {
    window: minifb::Window,
    pub gpu: gpu::GPU,
    height: usize,
    width: usize,
    scene: scene::Scene,
}

impl Window {
    pub fn new(width: usize, height: usize, scene: scene::Scene) -> Window {
        let gpu = block_on(gpu::GPU::new(width, height, &scene));
        let window = minifb::Window::new(
            "Minimal Renderer - ESC to exit",
            width,
            height,
            minifb::WindowOptions::default(),
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

        Window { gpu, window, height, width, scene }
    }

    pub async fn update(&mut self) {
        let buffer = self.gpu.execute_pipeline(self.width, self.height, &self.scene).await;

        // Update the window with the buffer
        self.window
            .update_with_buffer(&buffer, self.width, self.height)
            .expect("Failed to update window");
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub fn is_key_down(&self, key: minifb::Key) -> bool {
        self.window.is_key_down(key)
    }
}
