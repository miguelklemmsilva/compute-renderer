mod clear_pass;
mod fragment_pass;
pub mod gpu;
mod gpu_buffers;
mod raster_pass;
mod vertex_pass;

use crate::util::dispatch_size;
use clear_pass::ClearPass;
use fragment_pass::FragmentPass;
use gpu_buffers::GpuBuffers;
use raster_pass::RasterPass;
use vertex_pass::VertexPass;
