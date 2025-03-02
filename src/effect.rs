use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Effect {
    Wave(WaveEffect),
    EdgeMelt(EdgeMeltEffect),
    Voxelize(VoxelizeEffect),
    Mirage(MirageEffect), // New effect variant: Digital Mirage
}

#[derive(Debug, Clone)]
pub struct WaveEffect {
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub direction: WaveDirection,
    pub speed: f32,
}

#[derive(Debug, Clone)]
pub struct EdgeMeltEffect {
    pub amplitude: f32, // Should be clamped between 0.0 and 0.33
    pub phase: f32,
    pub speed: f32,
}

#[derive(Debug, Clone)]
pub struct VoxelizeEffect {
    pub voxel_size: f32,
    pub speed: f32,
    pub time: f32,
}

#[derive(Debug, Clone)]
pub struct MirageEffect {
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub speed: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum WaveDirection {
    Vertical,
    Horizontal,
    Radial,
}

#[allow(dead_code)]
impl Effect {
    pub fn update(&mut self, delta_time: Duration) {
        let dt = delta_time.as_secs_f32();
        match self {
            Effect::Wave(wave) => wave.update(dt),
            Effect::EdgeMelt(edge_melt) => edge_melt.update(dt),
            Effect::Voxelize(voxelize) => voxelize.update(dt),
            Effect::Mirage(mirage) => mirage.update(dt),
        }
    }

    // Factory functions for creating effects
    pub fn wave(amplitude: f32, frequency: f32, speed: f32, direction: WaveDirection) -> Self {
        Effect::Wave(WaveEffect {
            amplitude,
            frequency,
            phase: 0.0,
            direction,
            speed,
        })
    }

    pub fn edge_melt(amplitude: f32, speed: f32) -> Self {
        Effect::EdgeMelt(EdgeMeltEffect {
            amplitude: amplitude.clamp(0.0, 0.33), // Ensure amplitude is within valid range
            phase: 0.0,
            speed,
        })
    }

    pub fn voxelize(voxel_size: f32, speed: f32) -> Self {
        Effect::Voxelize(VoxelizeEffect {
            voxel_size,
            speed,
            time: 0.0,
        })
    }

    pub fn mirage(amplitude: f32, frequency: f32, speed: f32) -> Self {
        Effect::Mirage(MirageEffect {
            amplitude,
            frequency,
            phase: 0.0,
            speed,
        })
    }
}

// WaveEffect implementation
impl WaveEffect {
    pub fn update(&mut self, delta_time: f32) {
        self.phase += delta_time * self.speed;
    }
}

// EdgeMeltEffect implementation
impl EdgeMeltEffect {
    pub fn update(&mut self, delta_time: f32) {
        self.phase += delta_time * self.speed;
    }
}

// VoxelizeEffect implementation
impl VoxelizeEffect {
    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time * self.speed;
        let t = ((self.time - std::f32::consts::FRAC_PI_2).sin() + 1.0) * 0.5; // Normalized t starting at 0.0
        self.voxel_size = t * 1.0; // Adjust voxel size based on t
    }
}

// MirageEffect implementation
impl MirageEffect {
    pub fn update(&mut self, delta_time: f32) {
        // Increase phase over time to drive an animated screen distortion.
        self.phase += delta_time * self.speed;
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
            Effect::Wave(wave) => {
                self.effect_type = 1;
                self.param1 = wave.amplitude;
                self.param2 = wave.frequency;
                self.param3 = wave.phase;
                self.param4 = match wave.direction {
                    WaveDirection::Vertical => 0.0,
                    WaveDirection::Horizontal => 1.0,
                    WaveDirection::Radial => 2.0,
                };
            }
            Effect::EdgeMelt(edge_melt) => {
                self.effect_type = 2;
                self.param1 = edge_melt.amplitude;
                self.param2 = edge_melt.phase;
            }
            Effect::Voxelize(voxelize) => {
                self.effect_type = 3;
                self.param1 = voxelize.voxel_size;
            }
            Effect::Mirage(mirage) => {
                self.effect_type = 4; // New type for Mirage effect
                self.param1 = mirage.amplitude;
                self.param2 = mirage.frequency;
                self.param3 = mirage.phase;
                self.param4 = mirage.speed;
            }
        }
    }
}