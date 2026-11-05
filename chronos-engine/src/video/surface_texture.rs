//! Viewport and Multi-Page Parallax calculations for Video & Live Wallpapers

use chronos_common::config::ScreenFillMode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug)]
pub struct ParallaxViewport {
    screen_width: f32,
    screen_height: f32,
    video_width: f32,
    video_height: f32,
    fill_mode: ScreenFillMode,
    parallax_swipe_factor: f32,
    gyro_factor: f32,
    current_offset_x: f32, // 0.0 to 1.0 from launcher
    gyro_yaw: f32,         // device tilt yaw (-pi/4 to pi/4)
    gyro_pitch: f32,       // device tilt pitch (-pi/4 to pi/4)
}

impl ParallaxViewport {
    pub fn new(
        screen_width: f32,
        screen_height: f32,
        video_width: f32,
        video_height: f32,
        fill_mode: ScreenFillMode,
        parallax_swipe_factor: f32,
    ) -> Self {
        Self {
            screen_width,
            screen_height,
            video_width,
            video_height,
            fill_mode,
            parallax_swipe_factor,
            gyro_factor: 15.0,
            current_offset_x: 0.5,
            gyro_yaw: 0.0,
            gyro_pitch: 0.0,
        }
    }

    pub fn set_launcher_offset(&mut self, x_offset: f32) {
        self.current_offset_x = x_offset.clamp(0.0, 1.0);
    }

    pub fn set_gyro_tilt(&mut self, yaw: f32, pitch: f32) {
        let max_angle = std::f32::consts::FRAC_PI_4;
        self.gyro_yaw = yaw.clamp(-max_angle, max_angle);
        self.gyro_pitch = pitch.clamp(-max_angle, max_angle);
    }

    pub fn set_gyro_factor(&mut self, factor: f32) {
        self.gyro_factor = factor;
    }

    pub fn calculate_viewport(&self) -> ViewportRect {
        let gyro_shift_x = self.gyro_yaw * self.gyro_factor;
        let gyro_shift_y = self.gyro_pitch * self.gyro_factor;

        match self.fill_mode {
            ScreenFillMode::AspectFill => {
                let screen_aspect = self.screen_width / self.screen_height;
                let video_aspect = self.video_width / self.video_height;

                if video_aspect > screen_aspect {
                    // Video is wider than screen: shift horizontally with launcher swipe + gyro
                    let scale = self.screen_height / self.video_height;
                    let scaled_width = self.video_width * scale;
                    let extra_width = scaled_width - self.screen_width;
                    let parallax_shift =
                        (self.current_offset_x - 0.5) * extra_width * self.parallax_swipe_factor;
                    let x = -extra_width * 0.5 + parallax_shift + gyro_shift_x;
                    ViewportRect {
                        x,
                        y: gyro_shift_y,
                        width: scaled_width,
                        height: self.screen_height,
                    }
                } else {
                    let scale = self.screen_width / self.video_width;
                    let scaled_height = self.video_height * scale;
                    let extra_height = scaled_height - self.screen_height;
                    ViewportRect {
                        x: gyro_shift_x,
                        y: -extra_height * 0.5 + gyro_shift_y,
                        width: self.screen_width,
                        height: scaled_height,
                    }
                }
            }
            ScreenFillMode::FitLetterbox => {
                let scale = (self.screen_width / self.video_width)
                    .min(self.screen_height / self.video_height);
                let w = self.video_width * scale;
                let h = self.video_height * scale;
                ViewportRect {
                    x: (self.screen_width - w) * 0.5 + gyro_shift_x,
                    y: (self.screen_height - h) * 0.5 + gyro_shift_y,
                    width: w,
                    height: h,
                }
            }
            ScreenFillMode::CustomZoom {
                zoom,
                offset_x,
                offset_y,
            } => {
                let factor = zoom as f32 / 100.0;
                let w = self.screen_width * factor;
                let h = self.screen_height * factor;
                let parallax_x = (self.current_offset_x - 0.5) * 100.0 * self.parallax_swipe_factor;
                ViewportRect {
                    x: (self.screen_width - w) * 0.5 + offset_x as f32 + parallax_x + gyro_shift_x,
                    y: (self.screen_height - h) * 0.5 + offset_y as f32 + gyro_shift_y,
                    width: w,
                    height: h,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallax_viewport_aspect_fill() {
        let mut pv = ParallaxViewport::new(
            1080.0,
            2400.0,
            3840.0,
            2160.0,
            ScreenFillMode::AspectFill,
            0.5,
        );
        let vp_center = pv.calculate_viewport();
        assert!(vp_center.width > 1080.0);

        pv.set_launcher_offset(0.0);
        let vp_left = pv.calculate_viewport();

        pv.set_launcher_offset(1.0);
        let vp_right = pv.calculate_viewport();

        assert_ne!(vp_left.x, vp_right.x);
    }

    #[test]
    fn test_gyro_dynamic_movement() {
        let mut pv = ParallaxViewport::new(
            1080.0,
            2400.0,
            3840.0,
            2160.0,
            ScreenFillMode::AspectFill,
            0.5,
        );
        let vp_initial = pv.calculate_viewport();

        // Tilt phone to right (positive yaw)
        pv.set_gyro_tilt(0.3, 0.2);
        let vp_tilted = pv.calculate_viewport();

        assert!(vp_tilted.x > vp_initial.x);
        assert!(vp_tilted.y > vp_initial.y);
    }
}
