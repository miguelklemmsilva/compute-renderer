use crate::scene;

use super::{binning_pass::BinningPass, ClearPass, GpuBuffers, RasterPass};

pub struct GPU {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub buffers: GpuBuffers,

    pub clear_pass: ClearPass,
    pub raster_pass: RasterPass,
    pub binning_pass: BinningPass,
    pub render_pass: super::RenderPass,
}

impl GPU {
    pub async fn new(
        instance: wgpu::Instance,
        width: usize,
        height: usize,
        scene: &scene::Scene,
        surface: &wgpu::Surface<'_>,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Failed to find an appropriate adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: adapter.features(),
                    required_limits: adapter.limits(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let buffers = GpuBuffers::new(&device, width as u32, height as u32, scene);

        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let surface_caps = surface.get_capabilities(&adapter);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width as u32,
            height: height as u32,
            present_mode: wgpu::PresentMode::Immediate,
            desired_maximum_frame_latency: 1,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let clear_pass = ClearPass::new(&device, &buffers);
        let binning_pass = BinningPass::new(&device, &buffers);
        let raster_pass = RasterPass::new(&device, &buffers);
        let render_pass = super::RenderPass::new(&device, &buffers, format);

        Self {
            device,
            queue,
            config,
            buffers,
            clear_pass,
            raster_pass,
            binning_pass,
            render_pass,
        }
    }

    pub async fn execute_pipeline(&mut self, width: usize, height: usize, scene: &scene::Scene, surface: &wgpu::Surface<'_>) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        let frame = surface.get_current_texture().unwrap();

        self.clear_pass.execute(&mut encoder, width, height);
        self.binning_pass
            .execute(&mut encoder, scene, width as u32, height as u32);
        self.raster_pass
            .execute(&mut encoder, width as u32, height as u32, scene);
        self.render_pass.execute(&mut encoder, &frame, &self.buffers);

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn resize(&mut self, width: u32, height: u32, scene: &scene::Scene) {
        // Recreate buffers that depend on screen dimensions
        self.buffers = GpuBuffers::new(&self.device, width, height, scene);

        // Recreate passes with new buffers
        self.clear_pass = ClearPass::new(&self.device, &self.buffers);
        self.binning_pass = BinningPass::new(&self.device, &self.buffers);
        self.raster_pass = RasterPass::new(&self.device, &self.buffers);
    }
}