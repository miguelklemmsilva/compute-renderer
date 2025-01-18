use crate::scene;

use super::{
    binning_pass::BinningPass, ClearPass, FragmentPass, GpuBuffers, RasterPass, VertexPass,
};

pub struct GPU {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub buffers: GpuBuffers,

    pub clear_pass: ClearPass,
    pub vertex_pass: VertexPass,
    pub raster_pass: RasterPass,
    pub fragment_pass: FragmentPass,
    pub binning_pass: BinningPass,
}

impl GPU {
    pub async fn new(width: usize, height: usize, scene: &scene::Scene) -> Self {
        let instance = wgpu::Instance::default();
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

        let clear_pass = ClearPass::new(&device, &buffers);
        let vertex_pass = VertexPass::new(&device, &buffers);
        let binning_pass = BinningPass::new(&device, &buffers);
        let raster_pass = RasterPass::new(&device, &buffers);
        let fragment_pass = FragmentPass::new(&device, &buffers);

        Self {
            device,
            queue,
            buffers,
            clear_pass,
            vertex_pass,
            raster_pass,
            fragment_pass,
            binning_pass,
        }
    }

    pub async fn execute_pipeline(
        &mut self,
        width: usize,
        height: usize,
        scene: &scene::Scene,
    ) -> Vec<u32> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        // Calculate total number of indices
        let total_indices = scene
            .models
            .iter()
            .map(|m| {
                m.meshes
                    .iter()
                    .map(|mesh| mesh.indices.len())
                    .sum::<usize>()
            })
            .sum::<usize>() as u32;

        self.clear_pass.execute(&mut encoder, width, height);
        self.vertex_pass.execute(&mut encoder, total_indices);
        self.binning_pass
            .execute(&mut encoder, scene, width as u32, height as u32);
        self.raster_pass
            .execute(&mut encoder, width as u32, height as u32, scene);
        self.fragment_pass.execute(&mut encoder, width, height);

        self.queue.submit(Some(encoder.finish()));

        // Read the output buffer for the final pixels
        let buffer_slice = self.buffers.output_buffer.slice(..);
        let (tx, rx_output) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx_output.receive().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let pixels = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.buffers.output_buffer.unmap();

        pixels
    }
}
