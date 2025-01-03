# Software Rasterization Pipeline Using Shaders

A high-performance software rasterization pipeline implementation that demonstrates the flexibility and capabilities of compute shaders for real-time 3D rendering.

## Overview

This project implements a software rasterization pipeline using compute shaders to evaluate performance and flexibility compared to traditional hardware-accelerated pipelines. The implementation covers core pipeline stages including:

- Vertex Processing
- Rasterization
- Fragment Shading

The project aims to demonstrate scenarios where a custom shader-based approach may be advantageous over traditional hardware-accelerated rendering pipelines.

## Features

- Custom software rasterization pipeline
- Compute shader-based implementation
- Real-time 3D rendering capabilities
- Configurable scene management
- Dynamic lighting system
- Texture mapping support
- Performance benchmarking tools
- Stress testing functionality

## Requirements

- Rust (latest stable version)
- A GPU that supports compute shaders
- wgpu compatible graphics drivers

## Building

1. Clone the repository:
```bash
git clone [repository-url]
cd compute-renderer
```

2. Build the project:
```bash
cargo build --release
```

3. Run the project:
```bash
cargo run --release
```

## Project Structure

- `src/`
  - `scene.rs` - Scene management and object handling
  - `camera.rs` - Camera system implementation
  - `model.rs` - 3D model loading and processing
  - `shaders/` - WGSL shader implementations
  - `effect.rs` - Post-processing effects
  - `gpu.rs` - GPU interface and resource management
  - `raster_pass.rs` - Rasterization pipeline implementation

## Usage

The renderer supports various configurations through the `SceneConfig` struct, allowing you to:

- Load custom 3D models
- Apply textures
- Configure lighting
- Add post-processing effects
- Run performance stress tests

Example configuration:
```rust
let config = SceneConfig {
    model_path: "path/to/model.obj",
    texture_path: Some("path/to/texture.png"),
    lights: vec![
        ([5.0, 5.0, 5.0], [1.0, 1.0, 1.0], 1.0),
    ],
    // ... additional configuration
};
```

## Performance Testing

The project includes built-in stress testing capabilities for performance evaluation:

- Model duplication for scene complexity testing
- Configurable grid layouts
- Automated benchmarking tools
- Performance metrics collection

## Acknowledgments

This project is being developed as part of a third-year computer science project at Lancaster University, focusing on exploring alternative approaches to real-time 3D rendering using modern GPU compute capabilities. 