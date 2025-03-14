use crate::scene::{self, Scene};

use super::{
    binning_pass::BinningPass, present_pass::PresentPass, raster_pass::TILE_SIZE,
    util::dispatch_size, FragmentPass, GpuBuffers, RasterPass,
};

pub struct CustomRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub surface_config: wgpu::SurfaceConfiguration,

    pub buffers: GpuBuffers,

    pub binning_pass: BinningPass,
    pub raster_pass: RasterPass,
    pub fragment_pass: FragmentPass,

    pub present_pass: PresentPass,

    pub width: u32,
    pub height: u32,
}

impl CustomRenderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        scene: &Scene,
    ) -> Self {
        // Choose adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create device/queue
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

        let format = wgpu::TextureFormat::Bgra8Unorm;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &surface_config);

        // Create the GpuBuffers and passes
        let width = surface_config.width;
        let height = surface_config.height;
        let buffers = GpuBuffers::new(&device, width, height, scene);

        let binning_pass = BinningPass::new(&device, &buffers);
        let raster_pass = RasterPass::new(&device, &buffers);
        let fragment_pass = FragmentPass::new(&device, &buffers);

        // Create the final pass that samples from the output texture
        let present_pass = PresentPass::new(&device, &buffers);

        Self {
            device,
            queue,
            surface_config,
            buffers,
            binning_pass,
            raster_pass,
            fragment_pass,
            present_pass,
            width,
            height,
        }
    }

    pub async fn render(
        &mut self,
        surface: &wgpu::Surface<'_>,
        scene: &scene::Scene,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => return Err(e),
        };

        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        let num_tiles_x =
            (self.surface_config.width as usize + TILE_SIZE as usize - 1) / TILE_SIZE as usize;
        let num_tiles_y =
            (self.surface_config.height as usize + TILE_SIZE as usize - 1) / TILE_SIZE as usize;

        let total_tile_dispatch = dispatch_size((num_tiles_x * num_tiles_y) as u32);

        let total_pixel_dispatch =
            dispatch_size(self.surface_config.width * self.surface_config.height);

        self.binning_pass.execute(
            &mut encoder,
            scene.gx_tris,
            scene.gy_tris,
            total_tile_dispatch,
        );
        self.raster_pass.execute(
            &mut encoder,
            self.surface_config.width,
            self.surface_config.height,
        );
        self.fragment_pass
            .execute(&mut encoder, total_pixel_dispatch);

        self.present_pass
            .execute(&mut encoder, &frame_view);

        self.queue.submit(Some(encoder.finish()));

        frame.present();

        Ok(())
    }

    pub fn resize(&mut self, config: &wgpu::SurfaceConfiguration, scene: &Scene) {
        self.surface_config = config.clone();
        self.width = config.width;
        self.height = config.height;

        // Recreate the output texture and present pass
        self.buffers = GpuBuffers::new(&self.device, self.width, self.height, scene);
        self.binning_pass = BinningPass::new(&self.device, &self.buffers);
        self.raster_pass = RasterPass::new(&self.device, &self.buffers);
        self.fragment_pass = FragmentPass::new(&self.device, &self.buffers);
        self.present_pass
            .resize(&self.device, &self.buffers.output_view);
    }
}
