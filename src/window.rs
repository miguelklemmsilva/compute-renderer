use pixels::{Pixels, SurfaceTexture};
use std::{collections::HashSet, time::Duration};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window as WinitWindow, WindowAttributes, WindowId};

use crate::{
    custom_pipeline, performance::PerformanceCollector, scene,
    wgpu_pipeline::renderer::WgpuRenderer,
};

pub enum RenderBackend {
    WgpuPipeline {
        surface: wgpu::Surface<'static>,
        renderer: WgpuRenderer,
    },
    CustomPipeline {
        pixels: Pixels<'static>,
        gpu: custom_pipeline::gpu::GPU,
    },
}

pub struct Window {
    winit_window: Option<WinitWindow>,
    backend: Option<RenderBackend>,
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

    backend_type: BackendType,
}

impl ApplicationHandler for Window {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.winit_window = Some(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64)),
                )
                .unwrap(),
        );

        let window = self.winit_window.as_ref().unwrap();

        // Initialize the appropriate backend based on configuration
        match self.backend_type {
            BackendType::WgpuPipeline => {
                let instance = wgpu::Instance::default();
                // SAFETY: The window is stored in self.winit_window and will live as long as the surface
                let surface = unsafe {
                    let surface = instance.create_surface(window).unwrap();
                    std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
                };

                let renderer = pollster::block_on(WgpuRenderer::new(
                    &instance,
                    &surface,
                    self.width as u32,
                    self.height as u32,
                    &self.scene,
                ));

                self.backend = Some(RenderBackend::WgpuPipeline { surface, renderer });
            }
            BackendType::CustomPipeline => {
                let surface_texture =
                    SurfaceTexture::new(self.width as u32, self.height as u32, window);
                let pixels = unsafe {
                    // SAFETY: We know the window will outlive the pixels
                    std::mem::transmute::<Pixels<'_>, Pixels<'static>>(
                        Pixels::new(self.width as u32, self.height as u32, surface_texture)
                            .unwrap(),
                    )
                };
                let gpu = pollster::block_on(custom_pipeline::gpu::GPU::new(
                    self.width,
                    self.height,
                    &self.scene,
                ));

                self.backend = Some(RenderBackend::CustomPipeline { pixels, gpu });
            }
        }
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
            }
            WindowEvent::Resized(size) => {
                self.width = size.width as usize;
                self.height = size.height as usize;

                if let Some(camera) = self.scene.get_active_camera_mut() {
                    camera.set_aspect_ratio(size.width as f32 / size.height as f32);
                }

                if let Some(backend) = &mut self.backend {
                    match backend {
                        RenderBackend::WgpuPipeline { surface, renderer } => {
                            let mut config = renderer.config.clone();
                            config.width = size.width;
                            config.height = size.height;
                            surface.configure(&renderer.device, &config);
                            renderer.resize(&config);
                        }
                        RenderBackend::CustomPipeline { pixels, gpu } => {
                            if pixels.resize_surface(size.width, size.height).is_err() {
                                event_loop.exit();
                                return;
                            }
                            if pixels.resize_buffer(size.width, size.height).is_err() {
                                event_loop.exit();
                                return;
                            }
                            gpu.resize(size.width, size.height, &self.scene);
                        }
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

#[derive(Clone, Copy)]
pub enum BackendType {
    WgpuPipeline,
    CustomPipeline,
}

impl Window {
    /// Create the Window object
    pub fn new_with_window(
        width: usize,
        height: usize,
        scene: scene::Scene,
        collector: PerformanceCollector,
        backend_type: BackendType,
    ) -> Result<Window, Box<dyn std::error::Error>> {
        Ok(Window {
            winit_window: None,
            backend: None,
            backend_type,
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

        // Update backend type
        self.backend_type = scene_config.backend_type;

        // Recreate the backend with the new scene
        if let Some(window) = &self.winit_window {
            match self.backend_type {
                BackendType::WgpuPipeline => {
                    let instance = wgpu::Instance::default();
                    let surface = unsafe {
                        let surface = instance.create_surface(window).unwrap();
                        std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
                    };

                    let renderer = pollster::block_on(WgpuRenderer::new(
                        &instance,
                        &surface,
                        self.width as u32,
                        self.height as u32,
                        &self.scene,
                    ));

                    self.backend = Some(RenderBackend::WgpuPipeline { surface, renderer });
                }
                BackendType::CustomPipeline => {
                    let surface_texture =
                        SurfaceTexture::new(self.width as u32, self.height as u32, window);
                    let pixels = unsafe {
                        std::mem::transmute::<Pixels<'_>, Pixels<'static>>(
                            Pixels::new(self.width as u32, self.height as u32, surface_texture)
                                .unwrap(),
                        )
                    };
                    let gpu = pollster::block_on(custom_pipeline::gpu::GPU::new(
                        self.width,
                        self.height,
                        &self.scene,
                    ));

                    self.backend = Some(RenderBackend::CustomPipeline { pixels, gpu });
                }
            }
        }

        true
    }

    /// Update the application each frame
    pub async fn update(&mut self, delta_time: Duration) -> bool {
        let frame_time = self.last_frame_time.elapsed().as_secs_f64();
        self.last_frame_time = std::time::Instant::now();

        self.frame_times.push(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

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
                delta_time.as_secs_f32(),
            );
        }

        if let Some(backend) = &mut self.backend {
            match backend {
                RenderBackend::WgpuPipeline { surface, renderer } => {
                    match renderer.render(surface, &self.scene) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.winit_window {
                                let size = window.inner_size();
                                let mut config = renderer.config.clone();
                                config.width = size.width;
                                config.height = size.height;
                                surface.configure(&renderer.device, &config);
                                renderer.resize(&config);
                            }
                        }
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                RenderBackend::CustomPipeline { pixels, gpu } => {
                    self.scene.update(gpu, delta_time);
                    let buffer = gpu
                        .execute_pipeline(self.width, self.height, &self.scene)
                        .await;

                    let frame = pixels.frame_mut();
                    let buffer_size = buffer.len() * 4;
                    if frame.len() == buffer_size {
                        frame.copy_from_slice(unsafe {
                            std::slice::from_raw_parts(buffer.as_ptr() as *const u8, buffer_size)
                        });

                        if pixels.render().is_err() {
                            return false;
                        }
                    } else {
                        eprintln!(
                            "Buffer size mismatch: frame buffer size = {}, gpu buffer size = {}",
                            frame.len(),
                            buffer_size
                        );
                        return false;
                    }
                }
            }
        }

        !self.collector.update()
    }
}
