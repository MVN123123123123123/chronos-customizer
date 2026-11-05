//! AI Neural Depth Segmentation and 2.5D Layer Occlusion Engine

use chronos_common::config::DepthEffectConfig;

#[derive(Debug, Clone, Copy)]
pub struct GyroTilt {
    pub yaw: f32,   // Radians (-pi/2 to pi/2)
    pub pitch: f32, // Radians (-pi/2 to pi/2)
}

impl Default for GyroTilt {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayerOffsets {
    pub bg_offset_x: f32,
    pub bg_offset_y: f32,
    pub fg_offset_x: f32,
    pub fg_offset_y: f32,
}

#[derive(Debug)]
pub struct DepthMap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>, // 0.0 (closest) to 1.0 (furthest)
}

impl DepthMap {
    pub fn new_synthetic(width: u32, height: u32, subject_radius: f32) -> Self {
        let mut data = Vec::with_capacity((width * height) as usize);
        let cx = width as f32 * 0.5;
        let cy = height as f32 * 0.5;

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < subject_radius {
                    // Subject in foreground (depth 0.2)
                    data.push(0.2);
                } else {
                    // Background further away (depth 0.8)
                    data.push(0.8);
                }
            }
        }

        Self {
            width,
            height,
            data,
        }
    }

    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let x = ((u.clamp(0.0, 1.0) * (self.width - 1) as f32).round() as u32).min(self.width - 1);
        let y =
            ((v.clamp(0.0, 1.0) * (self.height - 1) as f32).round() as u32).min(self.height - 1);
        let idx = (y * self.width + x) as usize;
        self.data.get(idx).copied().unwrap_or(1.0)
    }

    /// Evaluates whether the clock should be occluded at coordinate (u, v)
    pub fn is_clock_occluded(&self, u: f32, v: f32, clock_z: f32) -> bool {
        let depth = self.sample(u, v);
        depth < clock_z
    }
}

pub struct DepthEngine {
    pub config: DepthEffectConfig,
    pub current_tilt: GyroTilt,
}

impl DepthEngine {
    pub fn new(config: DepthEffectConfig) -> Self {
        Self {
            config,
            current_tilt: GyroTilt::default(),
        }
    }

    pub fn update_gyro_tilt(&mut self, yaw: f32, pitch: f32) {
        // Clamp sensor inputs to safe 45-degree tilt limits
        let max_angle = std::f32::consts::FRAC_PI_4;
        self.current_tilt.yaw = yaw.clamp(-max_angle, max_angle);
        self.current_tilt.pitch = pitch.clamp(-max_angle, max_angle);
    }

    pub fn calculate_offsets(&self) -> LayerOffsets {
        if !self.config.enabled {
            return LayerOffsets {
                bg_offset_x: 0.0,
                bg_offset_y: 0.0,
                fg_offset_x: 0.0,
                fg_offset_y: 0.0,
            };
        }

        // Differential parallax: background shifts opposite to foreground
        let bg_offset_x = -self.config.parallax_k_bg * self.current_tilt.yaw;
        let bg_offset_y = -self.config.parallax_k_bg * self.current_tilt.pitch;
        let fg_offset_x = self.config.parallax_k_fg * self.current_tilt.yaw;
        let fg_offset_y = self.config.parallax_k_fg * self.current_tilt.pitch;

        LayerOffsets {
            bg_offset_x,
            bg_offset_y,
            fg_offset_x,
            fg_offset_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_map_occlusion() {
        let dm = DepthMap::new_synthetic(100, 100, 30.0);
        // Center should be foreground subject (depth 0.2)
        assert!(dm.is_clock_occluded(0.5, 0.5, 0.50));
        // Corner should be background (depth 0.8)
        assert!(!dm.is_clock_occluded(0.0, 0.0, 0.50));
    }

    #[test]
    fn test_parallax_offsets() {
        let mut engine = DepthEngine::new(DepthEffectConfig::default());
        engine.update_gyro_tilt(0.2, -0.1);
        let offsets = engine.calculate_offsets();
        assert!(offsets.bg_offset_x < 0.0);
        assert!(offsets.fg_offset_x > 0.0);
    }
}
