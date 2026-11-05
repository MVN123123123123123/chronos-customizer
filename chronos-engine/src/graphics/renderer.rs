//! Core Rendering Pipeline for OLED Display Preview, Lock Screen, and Live Wallpaper

use super::surface::{MockSurface, RenderSurface, SurfaceError};
use chronos_common::config::{BurnInConfig, ChronosConfig, LayerStateBehavior};
use chronos_common::state::{DisplayState, StateMachine, TransitionUniforms};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct BurnInShifter {
    config: BurnInConfig,
    last_shift: Instant,
    current_offset_x: i32,
    current_offset_y: i32,
    step_angle: f32,
}

impl BurnInShifter {
    pub fn new(config: BurnInConfig) -> Self {
        Self {
            config,
            last_shift: Instant::now(),
            current_offset_x: 0,
            current_offset_y: 0,
            step_angle: 0.0,
        }
    }

    pub fn update(&mut self) -> (i32, i32) {
        if !self.config.enabled {
            return (0, 0);
        }

        let elapsed = self.last_shift.elapsed();
        if elapsed >= Duration::from_secs(self.config.shift_interval_seconds as u64) {
            self.last_shift = Instant::now();
            // Circular orbit walk to avoid static burn-in hot-spots
            self.step_angle += std::f32::consts::FRAC_PI_4; // ~45 degrees step
            let r = self.config.max_shift_px as f32;
            self.current_offset_x = (r * self.step_angle.cos()).round() as i32;
            self.current_offset_y = (r * self.step_angle.sin()).round() as i32;
        }

        (self.current_offset_x, self.current_offset_y)
    }

    pub fn current_offset(&self) -> (i32, i32) {
        (self.current_offset_x, self.current_offset_y)
    }
}

pub struct ChronosRenderer {
    pub surface: Box<dyn RenderSurface>,
    pub state_machine: StateMachine,
    pub burn_in: BurnInShifter,
    pub config: ChronosConfig,
    last_render_time: Instant,
    current_refresh_rate: u32,
}

impl ChronosRenderer {
    pub fn new(surface: Box<dyn RenderSurface>, config: ChronosConfig) -> Self {
        let burn_in = BurnInShifter::new(config.aod.burn_in_protection.clone());
        let initial_state = DisplayState::ScreenOff;
        let mut sm = StateMachine::new(initial_state);
        sm.set_aod_timeout_seconds(config.aod.mode.timeout_seconds());

        let mut r = Self {
            surface,
            state_machine: sm,
            burn_in,
            config,
            last_render_time: Instant::now(),
            current_refresh_rate: 60,
        };
        r.update_refresh_rate(60);
        r
    }

    pub fn new_mock(width: u32, height: u32) -> Self {
        let surface = Box::new(MockSurface::new(width, height));
        Self::new(surface, ChronosConfig::default())
    }

    pub fn update_refresh_rate(&mut self, target_hz: u32) {
        if self.current_refresh_rate != target_hz {
            self.current_refresh_rate = target_hz;
            let _ = self.surface.set_frame_rate(target_hz as f32);
        }
    }

    pub fn get_active_layer_behaviors(&self) -> Vec<(String, LayerStateBehavior)> {
        let state = self.state_machine.current_state;
        self.config
            .wallpaper
            .multilayer_config
            .layers
            .iter()
            .map(|layer| {
                let behavior = match state {
                    DisplayState::DisplayPreview => layer.state_transitions.on_preview.clone(),
                    DisplayState::LockScreen => layer.state_transitions.on_lock.clone(),
                    DisplayState::HomeScreen => layer.state_transitions.on_home.clone(),
                    DisplayState::ScreenOff => LayerStateBehavior {
                        opacity: 0.0,
                        blur_radius_px: 0.0,
                        scale: 1.0,
                        hidden: true,
                        speed_multiplier: 0.0,
                    },
                };
                (layer.id.clone(), behavior)
            })
            .collect()
    }

    pub fn render_frame(&mut self) -> Result<TransitionUniforms, SurfaceError> {
        let now = Instant::now();
        let delta_ms = now.duration_since(self.last_render_time).as_millis() as u32;
        self.last_render_time = now;

        // 1. Update State Machine
        let (uniforms, timeout_event) = self.state_machine.tick(delta_ms);
        if let Some(ev) = timeout_event {
            self.state_machine.handle_event(ev);
        }

        // 2. Adjust panel refresh rate based on state (LTPO optimization)
        match self.state_machine.current_state {
            DisplayState::DisplayPreview => {
                if self.state_machine.active_transition.is_none() {
                    // Static AOD: negotiate lowest possible refresh rate (1Hz - 10Hz)
                    self.update_refresh_rate(self.config.aod.refresh_rate_hz);
                } else {
                    // Animating morph transition: keep 60Hz
                    self.update_refresh_rate(60);
                }
            }
            DisplayState::LockScreen | DisplayState::HomeScreen => {
                self.update_refresh_rate(60);
            }
            DisplayState::ScreenOff => {
                self.update_refresh_rate(1);
            }
        }

        // 3. Update Burn-In offsets
        let (shift_x, shift_y) = self.burn_in.update();

        // 4. OLED Black Subpixel Clear
        // In OLED AOD, all non-widget areas must be #000000 to conserve power
        let clear_color = if self.config.power.oled_true_black
            && self.state_machine.current_state == DisplayState::DisplayPreview
        {
            (0.0f32, 0.0f32, 0.0f32, 1.0f32)
        } else {
            (0.05f32, 0.05f32, 0.05f32, 1.0f32)
        };

        log::trace!(
            "Render Frame: state={:?}, clear={:?}, shift=({}, {}), brightness={}, layers={}",
            self.state_machine.current_state,
            clear_color,
            shift_x,
            shift_y,
            uniforms.brightness,
            self.config.wallpaper.multilayer_config.layers.len()
        );

        // 5. Swap buffers
        self.surface.swap_buffers()?;

        Ok(uniforms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_initialization_and_frame() {
        let mut renderer = ChronosRenderer::new_mock(1080, 2400);
        assert_eq!(renderer.surface.width(), 1080);
        assert_eq!(renderer.surface.height(), 2400);

        let uniforms = renderer.render_frame().expect("Render frame failed");
        assert_eq!(uniforms.brightness, 0.0); // ScreenOff

        let layers = renderer.get_active_layer_behaviors();
        assert!(!layers.is_empty());
    }

    #[test]
    fn test_burn_in_shifter() {
        let mut shifter = BurnInShifter::new(BurnInConfig {
            enabled: true,
            shift_interval_seconds: 0, // Instant shift for test
            max_shift_px: 3,
        });

        let (x, y) = shifter.update();
        assert!(x.abs() <= 3);
        assert!(y.abs() <= 3);
    }

    #[test]
    fn test_layer_transitions_and_aod_timeout() {
        use chronos_common::config::AodMode;
        use chronos_common::state::ScreenEvent;

        let mut config = ChronosConfig::default();
        config.aod.mode = AodMode::TimeoutSeconds(2); // 2-second timeout

        let mut renderer = ChronosRenderer::new(Box::new(MockSurface::new(1080, 2400)), config);
        assert_eq!(
            renderer.state_machine.current_state,
            DisplayState::ScreenOff
        );

        // Tap screen -> DisplayPreview
        renderer
            .state_machine
            .handle_event(ScreenEvent::ScreenTapped);
        assert_eq!(
            renderer.state_machine.current_state,
            DisplayState::DisplayPreview
        );

        let preview_layers = renderer.get_active_layer_behaviors();
        let bg_layer = preview_layers
            .iter()
            .find(|(id, _)| id == "layer_0_bg")
            .unwrap();
        assert_eq!(bg_layer.1.opacity, 0.15); // Monochromatic dim
        let particle_layer = preview_layers
            .iter()
            .find(|(id, _)| id == "layer_1_particles")
            .unwrap();
        assert!(particle_layer.1.hidden); // Hidden in AOD for battery saving

        // Tap screen -> LockScreen
        renderer
            .state_machine
            .handle_event(ScreenEvent::ScreenTapped);
        assert_eq!(
            renderer.state_machine.current_state,
            DisplayState::LockScreen
        );
        let lock_layers = renderer.get_active_layer_behaviors();
        let bg_lock = lock_layers
            .iter()
            .find(|(id, _)| id == "layer_0_bg")
            .unwrap();
        assert_eq!(bg_lock.1.opacity, 1.0); // Full vibrancy on lockscreen

        // Biometric unlock -> HomeScreen
        renderer
            .state_machine
            .handle_event(ScreenEvent::BiometricSuccess);
        assert_eq!(
            renderer.state_machine.current_state,
            DisplayState::HomeScreen
        );
        let home_layers = renderer.get_active_layer_behaviors();
        let bg_home = home_layers
            .iter()
            .find(|(id, _)| id == "layer_0_bg")
            .unwrap();
        assert_eq!(bg_home.1.blur_radius_px, 24.0); // Progressive launcher blur
    }
}
