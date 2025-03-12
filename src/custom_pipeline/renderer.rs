use std::time::Instant;

use crate::{
    custom_pipeline::util::{TileMeta, TriangleMeta},
    scene,
};

use super::{
    binning_pass::BinningPass, raster_pass::TILE_SIZE, util::dispatch_size, FragmentPass,
    GpuBuffers, RasterPass,
};

pub struct CustomRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub buffers: GpuBuffers,

    pub raster_pass: RasterPass,
    pub fragment_pass: FragmentPass,
    pub binning_pass: BinningPass,
}

impl CustomRenderer {
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

        let binning_pass = BinningPass::new(&device, &buffers);
        let raster_pass = RasterPass::new(&device, &buffers);
        let fragment_pass = FragmentPass::new(&device, &buffers);

        Self {
            device,
            queue,
            buffers,
            raster_pass,
            fragment_pass,
            binning_pass,
        }
    }

    pub async fn render(&mut self, width: usize, height: usize, scene: &scene::Scene) -> Vec<u32> {
        let mut encoder1 = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        let num_tiles_x = (width + TILE_SIZE as usize - 1) / TILE_SIZE as usize;
        let num_tiles_y = (height + TILE_SIZE as usize - 1) / TILE_SIZE as usize;

        let total_tile_dispatch = dispatch_size((num_tiles_x * num_tiles_y) as u32);

        let start_time = Instant::now();

        let binning_start = Instant::now();
        self.binning_pass.execute(
            &mut encoder1,
            scene.gx_tris,
            scene.gy_tris,
            total_tile_dispatch,
        );

        let (_, rx_output) = futures_intrusive::channel::shared::oneshot_channel::<
            Result<(), wgpu::BufferAsyncError>,
        >();

        self.queue.submit(Some(encoder1.finish()));
        self.device.poll(wgpu::Maintain::Wait);
        rx_output.receive().await;

        let binning_duration = binning_start.elapsed();

        let mut encoder2 = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        let raster_start = Instant::now();
        self.raster_pass
            .execute(&mut encoder2, width as u32, height as u32);

        let (_, rx_output) = futures_intrusive::channel::shared::oneshot_channel::<
            Result<(), wgpu::BufferAsyncError>,
        >();
        self.queue.submit(Some(encoder2.finish()));
        self.device.poll(wgpu::Maintain::Wait);
        rx_output.receive().await;
        let raster_duration = raster_start.elapsed();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        let fragment_start = Instant::now();
        self.fragment_pass
            .execute(&mut encoder, width as u32, height as u32);

        self.queue.submit(Some(encoder.finish()));

        // Read the output buffer for the final pixels
        let buffer_slice = self.buffers.output_buffer.slice(..);
        let (tx, rx_output) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(
            wgpu::MapMode::Read,
            move |result: Result<(), wgpu::BufferAsyncError>| {
                tx.send(result).unwrap();
            },
        );
        self.device.poll(wgpu::Maintain::Wait);
        rx_output.receive().await.unwrap().unwrap();

        let fragment_duration = fragment_start.elapsed();
        let end_time = Instant::now();
        let total_time = end_time.duration_since(start_time);

        println!("Binning Pass Time: {:?}", binning_duration);
        println!("Raster Pass Time: {:?}", raster_duration);
        println!("Fragment Pass Time: {:?}", fragment_duration);

        println!("Total Render Time: {:?}", total_time);

        let output_data = buffer_slice.get_mapped_range();
        let pixels = bytemuck::cast_slice(&output_data).to_vec();
        drop(output_data);
        self.buffers.output_buffer.unmap();
        pixels
    }

    #[allow(dead_code)]
    async fn scene_metrics(&self, width: usize, height: usize) {
        let num_tiles_x = (width + TILE_SIZE as usize - 1) / TILE_SIZE as usize;
        let num_tiles_y = (height + TILE_SIZE as usize - 1) / TILE_SIZE as usize;

        let triangle_meta_slice = self.buffers.triangle_meta_buffer.slice(..);
        let (tx, rx_triangle_meta) = futures_intrusive::channel::shared::oneshot_channel();
        triangle_meta_slice.map_async(
            wgpu::MapMode::Read,
            move |result: Result<(), wgpu::BufferAsyncError>| {
                tx.send(result).unwrap();
            },
        );
        self.device.poll(wgpu::Maintain::Wait);
        rx_triangle_meta.receive().await.unwrap().unwrap();

        let triangle_meta_data = triangle_meta_slice.get_mapped_range();
        let triangle_meta: Vec<TriangleMeta> = bytemuck::cast_slice(&triangle_meta_data).to_vec();
        drop(triangle_meta_data);
        self.buffers.triangle_meta_buffer.unmap();

        let tile_triangles_slice = self.buffers.tile_buffer.slice(..);
        let (tx, rx_tile_triangles) = futures_intrusive::channel::shared::oneshot_channel();
        tile_triangles_slice.map_async(
            wgpu::MapMode::Read,
            move |result: Result<(), wgpu::BufferAsyncError>| {
                tx.send(result).unwrap();
            },
        );
        self.device.poll(wgpu::Maintain::Wait);
        rx_tile_triangles.receive().await.unwrap().unwrap();

        let tile_triangles_data = tile_triangles_slice.get_mapped_range();
        let tile_triangles: Vec<TileMeta> = bytemuck::cast_slice(&tile_triangles_data).to_vec();
        drop(tile_triangles_data);
        self.buffers.tile_buffer.unmap();

        let mut total_triangle_area = 0.0;
        let mut non_culled_triangles = 0;

        // Loop over every triangle and accumulate its screen-space area if it was not culled.
        for tri in triangle_meta.iter() {
            // a triangle is culled if its tile_range is zero.
            let tile_count = tri.tile_range[0] * tri.tile_range[1];
            if tile_count > 0 {
                // Compute area from the clipped bounding box.
                let area = (tri.min_max[2] - tri.min_max[0]) * (tri.min_max[3] - tri.min_max[1]);
                total_triangle_area += area;
                non_culled_triangles += 1;
            }
        }

        // Calculate average triangle size.
        let avg_triangle_size = if non_culled_triangles > 0 {
            total_triangle_area / (non_culled_triangles as f32)
        } else {
            0.0
        };

        // Compute triangle coverage of the screen.
        let screen_area = (width * height) as f32;
        let triangle_coverage = total_triangle_area / screen_area;

        // Sum the number of triangles binned in each tile.
        let total_tiles = (num_tiles_x * num_tiles_y) as usize;
        let mut total_triangles_in_tiles = 0;
        for i in 0..total_tiles {
            total_triangles_in_tiles += tile_triangles[i].count;
        }
        let avg_triangles_per_tile = total_triangles_in_tiles as f32 / total_tiles as f32;

        println!("Triangle coverage: {:.2}", triangle_coverage);
        println!("Average triangles per tile: {:.2}", avg_triangles_per_tile);
        println!("Average triangle size: {:.2}", avg_triangle_size);
        println!("Non-culled triangles: {}", non_culled_triangles);
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
