use pixels::{Pixels, SurfaceTexture};
use std::{collections::HashSet, time::Duration};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window as WinitWindow, WindowAttributes, WindowId};

use crate::{gpu, performance::PerformanceCollector, scene};

pub struct Window {
    // Store the winit window so we can pass it to egui
    winit_window: Option<WinitWindow>,

    pixels: Option<Pixels<'static>>,
    pub gpu: gpu::gpu::GPU,
    pub height: usize,
    pub width: usize,
    pub scene: scene::Scene,
    pub keys_down: HashSet<KeyCode>,
    pub mouse_pressed: bool,
    pub collector: PerformanceCollector,

    last_frame_time: std::time::Instant,
    frame_times: Vec<f64>,

    // Scene cycling
    scene_configs: Vec<scene::SceneConfig>,
    current_scene_index: usize,
}

impl ApplicationHandler for Window {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let winit_window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64)),
            )
            .unwrap();

        // Create the pixels surface
        let surface_texture =
            SurfaceTexture::new(self.width as u32, self.height as u32, &winit_window);
        let pixels = unsafe {
            // SAFETY: We know the window will outlive the pixels
            std::mem::transmute::<Pixels<'_>, Pixels<'static>>(
                Pixels::new(self.width as u32, self.height as u32, surface_texture).unwrap(),
            )
        };

        self.winit_window = Some(winit_window);
        self.pixels = Some(pixels);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.collector.finalise();
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys_down.insert(keycode);
                            if keycode == KeyCode::Escape {
                                // Instead of exiting, try to load next scene
                                self.collector.finalise();
                                pollster::block_on(self.load_next_scene(event_loop));
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
            },
            WindowEvent::Resized(size) => {
                self.width = size.width as usize;
                self.height = size.height as usize;
                if let Some(pixels) = &mut self.pixels {
                    if pixels.resize_surface(size.width, size.height).is_err() {
                        event_loop.exit();
                    }
                }
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if self.mouse_pressed {
                    if let Some(camera) = self.scene.get_active_camera_mut() {
                        camera.process_mouse(delta.0 as f32, -delta.1 as f32);
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Update per frame
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Async block to call `self.update(delta_time).await`
        if pollster::block_on(async {
            if !self.update(delta_time).await {
                // Scene is done, try to load next scene
                self.collector.finalise();
                if !self.load_next_scene(event_loop).await {
                    event_loop.exit();
                    return Err(());
                }
            }
            Ok::<(), ()>(())
        })
        .is_err()
        {
            // If update returns false or fails
            event_loop.exit();
        }

        if let Some(window) = &self.winit_window {
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.collector.finalise();
    }
}

impl Window {
    /// Create the Window object
    pub fn new_with_window(
        width: usize,
        height: usize,
        scene: scene::Scene,
        collector: PerformanceCollector,
    ) -> Result<Window, Box<dyn std::error::Error>> {
        // Create GPU
        let gpu = pollster::block_on(gpu::gpu::GPU::new(width, height, &scene));

        Ok(Window {
            winit_window: None,
            pixels: None,
            gpu,
            height,
            width,
            scene,
            keys_down: HashSet::new(),
            mouse_pressed: false,
            collector,
            last_frame_time: std::time::Instant::now(),
            frame_times: Vec::with_capacity(100),
            scene_configs: Vec::new(),
            current_scene_index: 0,
        })
    }

    pub fn set_scene_configs(&mut self, configs: Vec<scene::SceneConfig>) {
        self.scene_configs = configs;
    }

    async fn load_next_scene(&mut self, event_loop: &ActiveEventLoop) -> bool {
        // Increment scene index
        self.current_scene_index += 1;

        // Check if we've gone through all scenes
        if self.current_scene_index >= self.scene_configs.len() {
            event_loop.exit();
            return false;
        }

        // Get the next scene config
        let scene_config = &self.scene_configs[self.current_scene_index];

        // Create new performance collector
        self.collector = PerformanceCollector::new(
            scene_config.name.clone(),
            self.current_scene_index,
            Duration::from_secs(scene_config.benchmark_duration_secs),
        );

        // Create new scene
        self.scene = crate::scene::Scene::from_config(
            scene_config,
            self.width as usize,
            self.height as usize,
        )
        .await;

        // Recreate GPU with new scene
        self.gpu = gpu::gpu::GPU::new(self.width, self.height, &self.scene).await;

        true
    }

    /// Update the application each frame
    pub async fn update(&mut self, delta_time: Duration) -> bool {
        // Calculate FPS
        let frame_time = self.last_frame_time.elapsed().as_secs_f64();
        self.last_frame_time = std::time::Instant::now();

        self.frame_times.push(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

        // Handle camera movement
        const BASE_MOVEMENT_SPEED: f32 = 2.0;
        let movement_speed = BASE_MOVEMENT_SPEED * delta_time.as_secs_f32();
        if let Some(camera) = self.scene.get_active_camera_mut() {
            camera.update_over_time(delta_time.as_secs_f32());
            camera.process_keyboard(
                self.keys_down.contains(&KeyCode::KeyW),
                self.keys_down.contains(&KeyCode::KeyS),
                self.keys_down.contains(&KeyCode::KeyA),
                self.keys_down.contains(&KeyCode::KeyD),
                self.keys_down.contains(&KeyCode::Space),
                self.keys_down.contains(&KeyCode::KeyC),
                self.keys_down.contains(&KeyCode::ShiftLeft),
                movement_speed,
            );
        }

        // Update scene
        self.scene.update(&mut self.gpu, delta_time);

        // Run the GPU compute/render pipeline
        let buffer = self
            .gpu
            .execute_pipeline(self.width, self.height, &self.scene)
            .await;

        // Copy the result into the pixel buffer
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            frame.copy_from_slice(unsafe {
                std::slice::from_raw_parts(buffer.as_ptr() as *const u8, buffer.len() * 4)
            });

            // Present our CPU buffer to the window
            if pixels.render().is_err() {
                return false;
            }
        }

        // Check if performance collector is done
        !self.collector.update()
    }
}
