use pixels::{Pixels, SurfaceTexture};
use std::{collections::HashSet, sync::Arc, time::Duration};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{self, Window as WinitWindow, WindowBuilder},
};

use crate::{gpu, performance::PerformanceCollector, scene};

pub struct Window {
    pixels: Option<Pixels<'static>>,
    pub gpu: gpu::gpu::GPU,
    pub height: usize,
    pub width: usize,
    pub scene: scene::Scene,
    pub last_mouse_pos: Option<(f32, f32)>,
    pub is_open: bool,
    pub keys_down: HashSet<KeyCode>,
    pub mouse_pressed: bool,
    pub collector: PerformanceCollector,
}

impl Window {
    pub fn new_with_window(
        width: usize,
        height: usize,
        scene: scene::Scene,
        winit_window: &WinitWindow,
        collector: PerformanceCollector,
    ) -> Result<Window, Box<dyn std::error::Error>> {
        let gpu = pollster::block_on(gpu::gpu::GPU::new(width, height, &scene));
        let surface_texture = SurfaceTexture::new(width as u32, height as u32, winit_window);
        let pixels = unsafe {
            // SAFETY: We know the window will live as long as the pixels instance
            std::mem::transmute::<Pixels<'_>, Pixels<'static>>(Pixels::new(
                width as u32,
                height as u32,
                surface_texture,
            )?)
        };

        Ok(Window {
            pixels: Some(pixels),
            gpu,
            height,
            width,
            scene,
            last_mouse_pos: None,
            is_open: true,
            keys_down: HashSet::new(),
            mouse_pressed: false,
            collector,
        })
    }

    fn try_recreate_pixels(&mut self, event_loop: &EventLoop<()>) -> bool {
        if let Ok(window_builder) = WindowBuilder::new()
            .with_title("Minimal Renderer - ESC to exit")
            .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64))
            .with_resizable(true)
            .build(event_loop)
        {
            let surface_texture =
                SurfaceTexture::new(self.width as u32, self.height as u32, window_builder);
            if let Ok(pixels) = Pixels::new(self.width as u32, self.height as u32, surface_texture)
            {
                self.pixels = Some(pixels);
                return true;
            }
        }
        false
    }

    pub async fn update(&mut self, delta_time: Duration) -> bool {
        if !self.is_open {
            return false;
        }

        if self.collector.update() {
            return false;
        }

        // Handle keyboard input with constant movement speed
        const BASE_MOVEMENT_SPEED: f32 = 2.0; // Units per second
        let movement_speed = BASE_MOVEMENT_SPEED * delta_time.as_secs_f32();


        if let Some(camera) = self.scene.get_active_camera_mut() {
            camera.update_over_time(delta_time.as_secs_f32());
            camera.process_keyboard(
                self.keys_down.contains(&KeyCode::KeyW),
                self.keys_down.contains(&KeyCode::KeyS),
                self.keys_down.contains(&KeyCode::KeyA),
                self.keys_down.contains(&KeyCode::KeyD),
                self.keys_down.contains(&KeyCode::Space),
                self.keys_down.contains(&KeyCode::ShiftLeft),
                movement_speed,
            );
        }

        self.scene.update(&mut self.gpu, delta_time);

        let buffer = self
            .gpu
            .execute_pipeline(self.width, self.height, &self.scene)
            .await;

        // Update the window with the buffer
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            frame.copy_from_slice(unsafe {
                std::slice::from_raw_parts(buffer.as_ptr() as *const u8, buffer.len() * 4)
            });

            if let Err(_) = pixels.render() {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    pub fn run_with_event_loop(
        mut self,
        event_loop: EventLoop<()>,
    ) {
        let mut last_frame_time = std::time::Instant::now();

        event_loop.set_control_flow(ControlFlow::Poll);

        let _ = event_loop.run(move |event, window_target| {
            if !self.is_open {
                window_target.exit();
                return;
            }

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        self.is_open = false;
                        window_target.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let PhysicalKey::Code(keycode) = event.physical_key {
                            match event.state {
                                ElementState::Pressed => {
                                    self.keys_down.insert(keycode);
                                    if keycode == KeyCode::Escape {
                                        self.is_open = false;
                                        window_target.exit();
                                    }
                                }
                                ElementState::Released => {
                                    self.keys_down.remove(&keycode);
                                }
                            }
                        }
                    }
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Left,
                        ..
                    } => {
                        self.mouse_pressed = state == ElementState::Pressed;
                    }
                    WindowEvent::Resized(size) => {
                        self.width = size.width as usize;
                        self.height = size.height as usize;
                        if let Some(pixels) = &mut self.pixels {
                            if let Err(_) = pixels.resize_surface(size.width, size.height) {
                                self.is_open = false;
                                window_target.exit();
                            }
                        }
                    }
                    _ => (),
                },
                Event::DeviceEvent {
                    event: winit::event::DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    if self.mouse_pressed {
                        if let Some(camera) = self.scene.get_active_camera_mut() {
                            camera.process_mouse(delta.0 as f32, -delta.1 as f32);
                        }
                    }
                }
                Event::AboutToWait => {
                    let now = std::time::Instant::now();
                    let delta_time = now.duration_since(last_frame_time);
                    last_frame_time = now;

                    if let Ok(()) = pollster::block_on(async {
                        if !self.update(delta_time).await {
                            window_target.exit();
                        }
                        Ok::<(), ()>(())
                    }) {
                        // Frame rendered successfully
                    } else {
                        self.is_open = false;
                        window_target.exit();
                    }
                }
                Event::LoopExiting => {
                    self.collector.finalise();
                }
                _ => (),
            }
        });
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn handle_key_press(&mut self, key: KeyCode) {
        self.keys_down.insert(key);
        if key == KeyCode::Escape {
            self.keys_down.clear();
        }
    }

    pub fn handle_key_release(&mut self, key: KeyCode) {
        self.keys_down.remove(&key);
    }

    pub fn cleanup(&mut self) {
        self.keys_down.clear();
        self.mouse_pressed = false;
        self.last_mouse_pos = None;
        self.pixels = None;
        self.is_open = false;
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.cleanup();
    }
}
