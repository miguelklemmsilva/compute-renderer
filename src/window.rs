use core::fmt;
use std::{collections::HashSet, time::Duration};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window as WinitWindow, WindowAttributes, WindowId};

use crate::custom_pipeline::renderer::CustomRenderer;
use crate::{performance::PerformanceCollector, scene, wgpu_pipeline::renderer::WgpuRenderer};

pub enum RenderBackend {
    WgpuPipeline { renderer: WgpuRenderer },
    CustomPipeline { renderer: CustomRenderer },
}

pub struct Window {
    winit_window: Option<WinitWindow>,
    backend: Option<RenderBackend>,
    surface: Option<wgpu::Surface<'static>>,
    pub height: usize,
    pub width: usize,
    pub scene: scene::Scene,
    pub keys_down: HashSet<KeyCode>,
    pub mouse_pressed: bool,
    pub collector: Option<PerformanceCollector>,

    // Scene cycling
    scene_configs: Vec<scene::SceneConfig>,
    current_scene_index: usize,

    backend_type: BackendType,
}

impl ApplicationHandler for Window {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // create performance collector for this scene
        self.collector = Some(PerformanceCollector::new(
            self.scene_configs[self.current_scene_index].scene_name(),
            self.current_scene_index,
            Duration::from_secs(
                self.scene_configs[self.current_scene_index].benchmark_duration_secs,
            ),
        ));

        self.winit_window = Some(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64)),
                )
                .unwrap(),
        );

        let window = self.winit_window.as_ref().unwrap();

        let window_name = &self.scene_configs[self.current_scene_index].scene_name();

        window.set_title(window_name);

        self.width = window.inner_size().width as usize;
        self.height = window.inner_size().height as usize;

        // wgpu uses its own surface struct
        let instance = wgpu::Instance::default();
        // SAFETY: The window is stored in self.winit_window and will live as long as the surface
        self.surface = Some(unsafe {
            let surface = instance.create_surface(window).unwrap();
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
        });

        match self.backend_type {
            BackendType::WgpuPipeline => {
                let renderer = pollster::block_on(WgpuRenderer::new(
                    &instance,
                    self.surface.as_ref().unwrap(),
                    self.width as u32,
                    self.height as u32,
                    &self.scene,
                ));

                self.backend = Some(RenderBackend::WgpuPipeline { renderer });
            }
            BackendType::CustomPipeline => {
                let renderer = pollster::block_on(CustomRenderer::new(
                    &instance,
                    self.surface.as_ref().unwrap(),
                    self.width as u32,
                    self.height as u32,
                    &self.scene,
                ));

                self.backend = Some(RenderBackend::CustomPipeline { renderer });
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.collector.as_mut().unwrap().finalise();
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys_down.insert(keycode);
                            // user switches scene with escape
                            match keycode {
                                KeyCode::Escape => {
                                    self.collector.as_mut().unwrap().finalise();
                                    pollster::block_on(self.load_next_scene(event_loop));
                                }
                                _ => {}
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
                        RenderBackend::WgpuPipeline { renderer } => {
                            let mut config = renderer.config.clone();
                            config.width = size.width;
                            config.height = size.height;
                            self.surface
                                .as_mut()
                                .unwrap()
                                .configure(&renderer.device, &config);
                            renderer.resize(&config);
                        }
                        RenderBackend::CustomPipeline { renderer } => {
                            let mut config = renderer.surface_config.clone();
                            config.width = size.width;
                            config.height = size.height;
                            self.surface
                                .as_mut()
                                .unwrap()
                                .configure(&renderer.device, &config);
                            renderer.resize(&config, &self.scene);
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
                // pan camera if user presses left click
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
        let delta_time = self.collector.as_mut().unwrap().last_frame_time.elapsed();
        self.collector.as_mut().unwrap().last_frame_time = std::time::Instant::now();

        // Async block to call `self.update(delta_time).await`
        if pollster::block_on(async {
            if !self.update(delta_time).await {
                // Scene is done, try to load next scene
                self.collector.as_mut().unwrap().finalise();
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
        self.collector.as_mut().unwrap().finalise();
    }
}

#[derive(Clone, Copy)]
pub enum BackendType {
    WgpuPipeline,
    CustomPipeline,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendType::WgpuPipeline => write!(f, "WGPU"),
            BackendType::CustomPipeline => write!(f, "Custom"),
        }
    }
}

impl Window {
    /// Create the Window object
    pub fn new_with_window(
        width: usize,
        height: usize,
        scene: scene::Scene,
        backend_type: BackendType,
    ) -> Result<Window, Box<dyn std::error::Error>> {
        Ok(Window {
            winit_window: None,
            surface: None,
            backend: None,
            backend_type,
            height,
            width,
            scene,
            keys_down: HashSet::new(),
            mouse_pressed: false,
            collector: None,
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
        self.collector = Some(PerformanceCollector::new(
            scene_config.scene_name(),
            self.current_scene_index,
            Duration::from_secs(scene_config.benchmark_duration_secs),
        ));

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
            window.set_title(&scene_config.scene_name());

            let instance = wgpu::Instance::default();
            self.surface = Some(unsafe {
                let surface = instance.create_surface(window).unwrap();
                std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
            });

            match self.backend_type {
                BackendType::WgpuPipeline => {
                    let renderer = pollster::block_on(WgpuRenderer::new(
                        &instance,
                        self.surface.as_ref().unwrap(),
                        self.width as u32,
                        self.height as u32,
                        &self.scene,
                    ));

                    self.backend = Some(RenderBackend::WgpuPipeline { renderer });
                }
                BackendType::CustomPipeline => {
                    let renderer = pollster::block_on(CustomRenderer::new(
                        &instance,
                        self.surface.as_ref().unwrap(),
                        self.width as u32,
                        self.height as u32,
                        &self.scene,
                    ));
                    self.backend = Some(RenderBackend::CustomPipeline { renderer });
                }
            }
        }

        true
    }

    /// Update the application each frame
    pub async fn update(&mut self, delta_time: Duration) -> bool {
        if let Some(camera) = self.scene.get_active_camera_mut() {
            camera.update_over_time(delta_time.as_secs_f32());
            camera.process_keyboard(&self.keys_down, delta_time.as_secs_f32());
        }

        if let Some(backend) = &mut self.backend {
            match backend {
                RenderBackend::WgpuPipeline { renderer } => {
                    match renderer
                        .render(self.surface.as_ref().unwrap(), &self.scene)
                        .await
                    {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.winit_window {
                                let size = window.inner_size();
                                let mut config = renderer.config.clone();
                                config.width = size.width;
                                config.height = size.height;
                                self.surface
                                    .as_mut()
                                    .unwrap()
                                    .configure(&renderer.device, &config);
                                renderer.resize(&config);
                            }
                        }
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                RenderBackend::CustomPipeline {
                    renderer: custom_renderer,
                } => {
                    // update scene info
                    self.scene.update_buffers(custom_renderer, delta_time);
                    // run the pipeline here
                    match custom_renderer
                        .render(self.surface.as_ref().unwrap(), &self.scene)
                        .await
                    {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.winit_window {
                                let size = window.inner_size();
                                let mut config = custom_renderer.surface_config.clone();
                                config.width = size.width;
                                config.height = size.height;
                                self.surface
                                    .as_mut()
                                    .unwrap()
                                    .configure(&custom_renderer.device, &config);
                                custom_renderer.resize(&config, &self.scene);
                            }
                        }
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
            }
        }

        !self.collector.as_mut().unwrap().update()
    }
}
