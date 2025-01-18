use egui::Context as EguiContext;
use egui_winit::winit::application::ApplicationHandler;
use egui_winit::winit::dpi::{LogicalSize, Size};
use egui_winit::winit::event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent};
use egui_winit::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use egui_winit::winit::keyboard::{KeyCode, PhysicalKey};
use egui_winit::winit::window::{Window as EguiWinitWindow, WindowAttributes, WindowId};
use egui_winit::State as EguiWinitState;
use pixels::{Pixels, SurfaceTexture};
use std::{collections::HashSet, time::Duration};

use crate::{gpu, performance::PerformanceCollector, scene};

pub struct Window {
    // Store the winit window so we can pass it to egui
    winit_window: Option<EguiWinitWindow>,

    pixels: Option<Pixels<'static>>,
    pub gpu: gpu::gpu::GPU,
    pub height: usize,
    pub width: usize,
    pub scene: scene::Scene,
    pub keys_down: HashSet<KeyCode>,
    pub mouse_pressed: bool,
    pub collector: PerformanceCollector,

    egui_state: Option<EguiWinitState>,

    last_frame_time: std::time::Instant,
    frame_times: Vec<f64>,
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

        // Create egui context
        let egui_ctx = EguiContext::default();
        let viewport_id = egui_ctx.viewport_id();

        // Create egui winit state
        let egui_state = EguiWinitState::new(egui_ctx, viewport_id, event_loop, None, None, None);

        self.winit_window = Some(winit_window);
        self.pixels = Some(pixels);
        self.egui_state = Some(egui_state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(winit_window) = &self.winit_window {
            if let Some(egui_state) = &mut self.egui_state {
                // First let egui handle the event
                let response = egui_state.on_window_event(winit_window, &event);
                let consumed = response.consumed;

                if !consumed {
                    // If egui didn't consume it, handle it ourselves
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
                                            event_loop.exit();
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
                        WindowEvent::CursorMoved { position, .. } => {
                            if self.mouse_pressed {
                                static mut LAST_X: f32 = 0.0;
                                static mut LAST_Y: f32 = 0.0;

                                unsafe {
                                    let x = position.x as f32;
                                    let y = position.y as f32;

                                    if LAST_X != 0.0 || LAST_Y != 0.0 {
                                        let x_offset = x - LAST_X;
                                        let y_offset = LAST_Y - y; // Reversed since y-coordinates go from bottom to top

                                        if let Some(camera) = self.scene.get_active_camera_mut() {
                                            camera.process_mouse(x_offset, y_offset);
                                        }
                                    }

                                    LAST_X = x;
                                    LAST_Y = y;
                                }
                            }
                        }
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
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: egui_winit::winit::event::DeviceId,
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
                event_loop.exit();
                return Err(());
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
        // Finalize performance stats if needed
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
            egui_state: None,
            last_frame_time: std::time::Instant::now(),
            frame_times: Vec::with_capacity(100),
        })
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

            if let Some(winit_window) = &self.winit_window {
                if let Some(egui_state) = &mut self.egui_state {
                    // --- EGUI PASS (logic only, no actual drawing to the pixel buffer) ---

                    // Gather egui input
                    let raw_input = egui_state.take_egui_input(winit_window);

                    // Run egui
                    let full_output = egui_state.egui_ctx().run(raw_input, |ctx| {
                        // Build your UI here
                        egui::Window::new("Performance")
                            .default_pos([10.0, 10.0])
                            .show(ctx, |ui| {
                                if !self.frame_times.is_empty() {
                                    let avg_frame_time = self.frame_times.iter().sum::<f64>()
                                        / self.frame_times.len() as f64;
                                    let fps = 1.0 / avg_frame_time;
                                    ui.label(format!("FPS: {:.1}", fps));
                                }
                                ui.label("Put your sliders/buttons here");
                            });
                    });

                    // Handle platform output (e.g. copy/paste)
                    egui_state.handle_platform_output(winit_window, full_output.platform_output);

                    // We *could* tessellate shapes here:
                    let shapes = full_output.shapes;
                    let _paint_jobs = egui_state.egui_ctx().tessellate(shapes, 1.0);
                }
            }

            // Present our CPU buffer to the window
            if pixels.render().is_err() {
                return false;
            }
        }

        // Check if performance collector is done
        !self.collector.update()
    }
}
