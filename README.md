# Software Rendering Pipeline Using Shaders

A high-performance software rendering pipeline implementation that demonstrates the flexibility and capabilities of compute shaders for real-time 3D rendering.

## Overview

This project implements a pipeline using compute shaders to evaluate performance and flexibility compared to traditional hardware-accelerated pipelines. The implementation covers core pipeline stages including:

- Vertex Processing
- Binning
- Rasterisation
- Fragment Shading

The project aims to demonstrate scenarios where a custom shader-based approach may be advantageous over traditional hardware-accelerated rendering pipelines.

![image](https://github.com/user-attachments/assets/0a254b94-a45a-41be-9125-06b54b773884)


## Features

- Custom software rendering pipeline
- Compute shader-based implementation
- Real-time 3D rendering capabilities
- Configurable scene management
- Dynamic lighting system
- Performance benchmarking tools

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

## Controls

- WASD to move around
- Hold left click to pan the camera
- SPACE/C to ascend/descend
- Shift to temporarily increase velocity
- [/] to permanently increase/decrease velocity

## Usage

The renderer supports various configurations through the `SceneConfig` struct, allowing you to:

- Load custom 3D models
- Configure lighting
- Add effects
- Use the built-in renderer or custom
- Choose free camera mode or orbital camera

Example configuration:

```rust
let config = SceneConfig {
    model_path: "path/to/model.obj",
    lights: vec![
        ([5.0, 5.0, 5.0], [1.0, 1.0, 1.0], 1.0),
    ],
    // ... additional configuration
};
```

## Releases

You can find the latest releases on the [GitHub Releases page](https://github.com/miguel4521/compute-renderer/releases).

To use the latest release:

1. Download the appropriate binary for your platform from the releases page
2. Extract the archive
3. Run the executable from the terminal/command prompt

For development, it's recommended to build from source following the instructions in the Building section above.

## Acknowledgments

This project is being developed as part of a third-year computer science project at Lancaster University, focusing on exploring alternative approaches to real-time 3D rendering using modern GPU compute capabilities. 
