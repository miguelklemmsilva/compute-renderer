use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Effect {
    Wave(WaveEffect),
    EdgeMelt(EdgeMeltEffect),
    Voxelize(VoxelizeEffect),
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
    pub time: f32,
}

#[derive(Debug, Clone)]
pub struct VoxelizeEffect {
    pub grid_size: f32,
    pub min_size: f32,
    pub max_size: f32,
    pub speed: f32,
    pub time: f32,
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
            Effect::Wave(wave) => wave.update(dt),
            Effect::EdgeMelt(edge_melt) => edge_melt.update(dt),
            Effect::Voxelize(voxelize) => voxelize.update(dt),
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
            time: 0.0,
        })
    }

    pub fn voxelize(min_size: f32, max_size: f32, speed: f32) -> Self {
        Effect::Voxelize(VoxelizeEffect {
            grid_size: min_size,
            min_size,
            max_size,
            speed,
            time: 0.0,
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
        let t = (self.time.sin() + 1.0) * 0.5;
        self.grid_size = self.min_size + (self.max_size - self.min_size) * t;
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
                self.effect_type = 5;
                self.param1 = voxelize.grid_size;
            }
        }
    }
}