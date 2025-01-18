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
pub enum CameraMode {
    Orbit,
    FirstPerson,
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub mode: CameraMode,
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub movement_speed: f32,
    pub mouse_sensitivity: f32,
}

impl Camera {
    const ZFAR: f32 = 1000.0;
    const ZNEAR: f32 = 0.1;
    const FOVY: f32 = std::f32::consts::PI / 2.0;
    const UP: Vec3 = Vec3::Y;

    pub fn new(distance: f32, theta: f32, phi: f32, target: Vec3, aspect: f32) -> Self {
        let mut camera = Self {
            mode: CameraMode::Orbit,
            eye: Vec3::ZERO,
            target,
            up: Self::UP,
            aspect,
            yaw: theta,
            pitch: phi,
            movement_speed: 5.0,
            mouse_sensitivity: 0.1,
        };

        // Calculate initial eye position
        let pitch_cos = phi.cos();
        let x = distance * theta.cos() * pitch_cos;
        let y = distance * phi.sin();
        let z = distance * theta.sin() * pitch_cos;
        camera.eye = Vec3::new(x, y, z);

        camera
    }

    pub fn new_first_person(position: Vec3, aspect: f32) -> Self {
        Self {
            mode: CameraMode::FirstPerson,
            eye: position,
            target: position - Vec3::Z,
            up: Self::UP,
            aspect,
            yaw: -90.0,
            pitch: 0.0,
            movement_speed: 5.0,
            mouse_sensitivity: 0.1,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(Self::FOVY, self.aspect, Self::ZNEAR, Self::ZFAR);
        proj * view
    }

    pub fn process_keyboard(
        &mut self,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
        shift: bool,
        delta_time: f32,
    ) {
        if let CameraMode::FirstPerson = self.mode {
            let velocity = self.movement_speed * delta_time * if shift { 10.0 } else { 1.0 };
            let front = (self.target - self.eye).normalize();
            let right_vec = front.cross(self.up).normalize();

            let mut movement = Vec3::ZERO;
            if forward {
                movement += front;
            }
            if backward {
                movement -= front;
            }
            if right {
                movement += right_vec;
            }
            if left {
                movement -= right_vec;
            }
            if up {
                movement += Vec3::Y;
            }
            if down {
                movement -= Vec3::Y;
            }

            if movement != Vec3::ZERO {
                movement = movement.normalize() * velocity;
                self.eye += movement;
                self.target += movement;
            }
        }
    }

    pub fn process_mouse(&mut self, x_offset: f32, y_offset: f32) {
        if let CameraMode::FirstPerson = self.mode {
            self.yaw += x_offset * self.mouse_sensitivity;
            self.pitch += y_offset * self.mouse_sensitivity;

            // Constrain pitch
            self.pitch = self.pitch.clamp(-89.0, 89.0);

            // Update target based on new angles
            let pitch_rad = self.pitch.to_radians();
            let yaw_rad = self.yaw.to_radians();

            let front = Vec3::new(
                yaw_rad.cos() * pitch_rad.cos(),
                pitch_rad.sin(),
                yaw_rad.sin() * pitch_rad.cos(),
            )
            .normalize();

            self.target = self.eye + front;
        }
    }

    pub fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    pub fn update_over_time(&mut self, delta_time: f32) {
        if let CameraMode::Orbit = self.mode {
            self.yaw += delta_time * 0.5; // Rotate around target

            // Calculate new eye position
            let pitch_cos = self.pitch.cos();
            let distance = (self.eye - self.target).length();
            let x = distance * self.yaw.cos() * pitch_cos;
            let y = distance * self.pitch.sin();
            let z = distance * self.yaw.sin() * pitch_cos;

            self.eye = Vec3::new(x, y, z) + self.target;
        }
    }
}
