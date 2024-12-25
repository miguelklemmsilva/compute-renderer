use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct CameraUniform {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub zoom: f32,
    pub target: Vec3,
    pub eye: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub up: Vec3,
    pub aspect: f32,
    pub time: f32,
}

impl Camera {
    const ZFAR: f32 = 100.;
    const ZNEAR: f32 = 0.1;
    const FOVY: f32 = std::f32::consts::PI / 2.0;
    const UP: Vec3 = Vec3::Y;

    pub fn new(zoom: f32, pitch: f32, yaw: f32, target: Vec3, aspect: f32) -> Self {
        let mut camera = Self {
            zoom,
            pitch,
            yaw,
            eye: Vec3::ZERO,
            target,
            up: Self::UP,
            aspect,
            time: 0.0,
        };
        camera.update();
        camera
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(Self::FOVY, self.aspect, Self::ZNEAR, Self::ZFAR);
        proj * view
    }

    pub fn update(&mut self) {
        let pitch_cos = self.pitch.cos();

        // Calculate the new position of the camera along an elliptical orbit
        let radius = self.zoom;
        let x = radius * self.yaw.cos() * pitch_cos;
        let y = radius * self.pitch.sin();
        let z = radius * self.yaw.sin() * pitch_cos;

        // Update the eye position
        self.eye = Vec3::new(x, y, z) + self.target;
    }

    pub fn update_over_time(&mut self, delta_time: f32) {
        // Update the time variable
        self.time += delta_time;

        // Rotate the camera around the target over time
        // Complete one rotation every 8 seconds
        const ROTATION_SPEED: f32 = 2.0 * std::f32::consts::PI / 8.0;
        self.yaw = self.time * ROTATION_SPEED;

        // Add a gentle bobbing motion in pitch
        // Oscillate between -0.2 and 0.2 radians with a 4 second period
        const PITCH_AMPLITUDE: f32 = 0.2;
        const PITCH_FREQUENCY: f32 = 2.0 * std::f32::consts::PI / 4.0;
        self.pitch = PITCH_AMPLITUDE * (self.time * PITCH_FREQUENCY).sin();

        // Ensure the camera is always looking at the target
        self.up = Vec3::Y;

        // Update the camera's position based on the new parameters
        self.update();
    }

    pub fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }
}
