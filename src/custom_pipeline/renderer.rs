use crate::scene;

use super::{
    binning_pass::BinningPass, raster_pass::TILE_SIZE, render_pass::RenderPass, util::dispatch_size, FragmentPass, GpuBuffers, RasterPass
};

pub struct CustomRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub buffers: GpuBuffers,

    pub raster_pass: RasterPass,
    pub binning_pass: BinningPass,
    pub fragment_pass: FragmentPass,
    pub render_pass: RenderPass,
}

impl CustomRenderer {
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

        let binning_pass = BinningPass::new(&device, &buffers);
        let raster_pass = RasterPass::new(&device, &buffers);
        let fragment_pass = FragmentPass::new(&device, &buffers);
        let render_pass = RenderPass::new(&device, &buffers, format);

        Self {
            device,
            queue,
            config,
            buffers,
            raster_pass,
            binning_pass,
            fragment_pass,
            render_pass,
        }
    }

    pub async fn execute_pipeline(
        &mut self,
        width: usize,
        height: usize,
        scene: &scene::Scene,
        surface: &wgpu::Surface<'_>,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        let num_tiles_x = (width + TILE_SIZE as usize - 1) / TILE_SIZE as usize;
        let num_tiles_y = (height + TILE_SIZE as usize - 1) / TILE_SIZE as usize;

        let total_tile_dispatch = dispatch_size((num_tiles_x * num_tiles_y) as u32);

        let total_pixel_dispatch = dispatch_size((width * height) as u32);

        self.binning_pass.execute(
            &mut encoder,
            scene.gx_tris,
            scene.gy_tris,
            total_tile_dispatch,
        );
        self.raster_pass
            .execute(&mut encoder, width as u32, height as u32);
        self.fragment_pass
            .execute(&mut encoder, total_pixel_dispatch);
        self.render_pass
            .execute(&mut encoder, &frame, &self.buffers);

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    pub fn resize(&mut self, width: u32, height: u32, scene: &scene::Scene) {
        // Recreate buffers that depend on screen dimensions
        self.buffers = GpuBuffers::new(&self.device, width, height, scene);

        // Recreate passes with new buffers
        self.binning_pass = BinningPass::new(&self.device, &self.buffers);
        self.raster_pass = RasterPass::new(&self.device, &self.buffers);
        self.fragment_pass = FragmentPass::new(&self.device, &self.buffers);
    }
}
