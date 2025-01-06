mod clear_pass;
mod vertex_pass;
mod raster_pass;
mod fragment_pass;
pub mod gpu;
mod gpu_buffers;

use gpu_buffers::GpuBuffers;
use clear_pass::ClearPass;
use vertex_pass::VertexPass;
use raster_pass::RasterPass;
use fragment_pass::FragmentPass;
use crate::util::dispatch_size;