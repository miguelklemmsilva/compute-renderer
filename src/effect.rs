use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Effect {
    Wave {
        amplitude: f32,
        frequency: f32,
        phase: f32,
        direction: WaveDirection,
        speed: f32,
    },
    Dissolve {
        threshold: f32,
        noise_scale: f32,
        speed: f32,
        loop_time: f32,
        time: f32,
    },
    SmoothToFlat {
        progress: f32,
        speed: f32,
        loop_time: f32,
        time: f32,
    },
    Pixelate {
        pixel_size: f32,
        min_size: f32,
        max_size: f32,
        speed: f32,
        time: f32,
    },
    Voxelize {
        grid_size: f32,
        min_size: f32,
        max_size: f32,
        speed: f32,
        time: f32,
    },
    CrossHatch {
        line_spacing: f32,
        line_width: f32,
        time: f32,
    },
}

#[derive(Debug, Clone)]
pub enum WaveDirection {
    Vertical,
    Horizontal,
    Radial,
}

impl Effect {
    pub fn update(&mut self, delta_time: Duration) {
        let dt = delta_time.as_secs_f32();
        match self {
            Effect::Wave { phase, speed, .. } => {
                *phase += dt * *speed;
            }
            Effect::Dissolve {
                threshold,
                speed,
                loop_time,
                time,
                ..
            } => {
                *time = (*time + dt) % *loop_time;
                let cycle_progress = (*time / *loop_time) * std::f32::consts::PI * 2.0;
                *threshold = ((cycle_progress.sin() + 1.0) * 0.5) * *speed;
            }
            Effect::SmoothToFlat {
                progress,
                speed,
                loop_time,
                time,
                ..
            } => {
                *time = (*time + dt) % *loop_time;
                let cycle_progress = (*time / *loop_time) * std::f32::consts::PI * 2.0;
                *progress = ((cycle_progress.sin() + 1.0) * 0.5) * *speed;
            }
            Effect::Pixelate {
                pixel_size,
                min_size,
                max_size,
                speed,
                time,
            } => {
                *time += dt * *speed;
                let t = (time.sin() + 1.0) * 0.5;
                *pixel_size = *min_size + (*max_size - *min_size) * t;
            }
            Effect::Voxelize {
                grid_size,
                min_size,
                max_size,
                speed,
                time,
            } => {
                *time += dt * *speed;
                let t = (time.sin() + 1.0) * 0.5;
                *grid_size = *min_size + (*max_size - *min_size) * t;
            }
            Effect::CrossHatch { time, .. } => {
                *time += dt;
            }
        }
    }

    pub fn wave_vertical(amplitude: f32, frequency: f32, speed: f32) -> Self {
        Effect::Wave {
            amplitude,
            frequency,
            phase: 0.0,
            direction: WaveDirection::Vertical,
            speed,
        }
    }

    pub fn wave_horizontal(amplitude: f32, frequency: f32, speed: f32) -> Self {
        Effect::Wave {
            amplitude,
            frequency,
            phase: 0.0,
            direction: WaveDirection::Horizontal,
            speed,
        }
    }

    pub fn wave_radial(amplitude: f32, frequency: f32, speed: f32) -> Self {
        Effect::Wave {
            amplitude,
            frequency,
            phase: 0.0,
            direction: WaveDirection::Radial,
            speed,
        }
    }

    pub fn dissolve(noise_scale: f32, speed: f32, loop_time: f32) -> Self {
        Effect::Dissolve {
            threshold: 0.0,
            noise_scale,
            speed,
            loop_time,
            time: 0.0,
        }
    }

    pub fn smooth_to_flat(speed: f32, loop_time: f32) -> Self {
        Effect::SmoothToFlat {
            progress: 0.0,
            speed,
            loop_time,
            time: 0.0,
        }
    }

    pub fn pixelate(min_size: f32, max_size: f32, speed: f32) -> Self {
        Effect::Pixelate {
            pixel_size: min_size,
            min_size,
            max_size,
            speed,
            time: 0.0,
        }
    }

    pub fn voxelize(min_size: f32, max_size: f32, speed: f32) -> Self {
        Effect::Voxelize {
            grid_size: min_size,
            min_size,
            max_size,
            speed,
            time: 0.0,
        }
    }

    pub fn cross_hatch(line_spacing: f32, line_width: f32, time: f32) -> Self {
        Effect::CrossHatch {
            line_spacing,
            line_width,
            time,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EffectUniform {
    pub effect_type: u32,
    pub param1: f32,
    pub param2: f32,
    pub param3: f32,
    pub param4: f32,
    pub time: f32,
    _padding: [f32; 2],
}

impl Default for EffectUniform {
    fn default() -> Self {
        Self {
            effect_type: 0,
            param1: 0.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            time: 0.0,
            _padding: [0.0; 2],
        }
    }
}

impl EffectUniform {
    pub fn update(&mut self, effect: &Effect, time: f32) {
        self.time = time;
        match effect {
            Effect::Wave {
                amplitude,
                frequency,
                phase,
                direction,
                speed: _,
            } => {
                self.effect_type = 1;
                self.param1 = *amplitude;
                self.param2 = *frequency;
                self.param3 = *phase;
                self.param4 = match direction {
                    WaveDirection::Vertical => 0.0,
                    WaveDirection::Horizontal => 1.0,
                    WaveDirection::Radial => 2.0,
                };
            }
            Effect::Dissolve {
                threshold,
                noise_scale,
                speed: _,
                loop_time: _,
                time: _,
            } => {
                self.effect_type = 2;
                self.param1 = *threshold;
                self.param2 = *noise_scale;
            }
            Effect::SmoothToFlat {
                progress,
                speed: _,
                loop_time: _,
                time: _,
            } => {
                self.effect_type = 3;
                self.param1 = *progress;
            }
            Effect::Pixelate {
                pixel_size,
                min_size: _,
                max_size: _,
                speed: _,
                time: _,
            } => {
                self.effect_type = 4;
                self.param1 = *pixel_size;
            }
            Effect::Voxelize {
                grid_size,
                min_size: _,
                max_size: _,
                speed: _,
                time: _,
            } => {
                self.effect_type = 5;
                self.param1 = *grid_size;
            }
            Effect::CrossHatch {
                line_spacing,
                line_width,
                time: _,
            } => {
                self.effect_type = 6;
                self.param1 = *line_spacing;
                self.param2 = *line_width;
            }
        }
    }
}
