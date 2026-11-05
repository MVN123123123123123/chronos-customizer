use crate::config::{TransitionEasing, TransitionStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DisplayState {
    #[default]
    ScreenOff,
    DisplayPreview, // Ambient / AOD
    LockScreen,
    HomeScreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenEvent {
    ScreenTurnedOff,
    ProximityDetected,
    ScreenTapped,
    PowerButtonPressed,
    BiometricStarted,
    BiometricSuccess,
    BiometricFailure,
    KeyguardDismissed,
    HomeScreenSwiped,
    TimeoutExpired,
    LockRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayTransition {
    pub from: DisplayState,
    pub to: DisplayState,
    pub progress: f32, // 0.0 to 1.0
    pub duration_ms: u32,
    pub style: TransitionStyle,
    pub easing: TransitionEasing,
}

impl DisplayTransition {
    pub fn new(
        from: DisplayState,
        to: DisplayState,
        duration_ms: u32,
        style: TransitionStyle,
        easing: TransitionEasing,
    ) -> Self {
        Self {
            from,
            to,
            progress: 0.0,
            duration_ms,
            style,
            easing,
        }
    }

    pub fn advance(&mut self, delta_ms: u32) -> bool {
        if self.duration_ms == 0 {
            self.progress = 1.0;
            return true;
        }
        self.progress += delta_ms as f32 / self.duration_ms as f32;
        if self.progress >= 1.0 {
            self.progress = 1.0;
            true
        } else {
            false
        }
    }

    /// Progress mapped through selected easing curve
    pub fn eased_progress(&self) -> f32 {
        let x = self.progress.clamp(0.0, 1.0);
        match self.easing {
            TransitionEasing::Linear => x,
            TransitionEasing::SmoothStep => x * x * (3.0 - 2.0 * x),
            TransitionEasing::EaseInOut => 0.5 * (1.0 - (std::f32::consts::PI * x).cos()),
            TransitionEasing::Spring => {
                // Gentle overshoot spring: -2^(-10x) * sin((x*10 - 0.75)*2*pi/3) + 1
                let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                if x == 0.0 {
                    0.0
                } else if x == 1.0 {
                    1.0
                } else {
                    (-10.0 * x).exp2() * ((x * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TransitionUniforms {
    pub brightness: f32,
    pub saturation: f32,
    pub blur_radius_px: f32,
    pub clock_scale: f32,
    pub clock_alpha: f32,
    pub glow_intensity: f32,
    pub parallax_intensity: f32,
    pub morph_progress: f32,
}

impl TransitionUniforms {
    pub fn for_state(state: DisplayState) -> Self {
        match state {
            DisplayState::ScreenOff => Self {
                brightness: 0.0,
                saturation: 0.0,
                blur_radius_px: 0.0,
                clock_scale: 0.8,
                clock_alpha: 0.0,
                glow_intensity: 0.0,
                parallax_intensity: 0.0,
                morph_progress: 0.0,
            },
            DisplayState::DisplayPreview => Self {
                brightness: 0.15,
                saturation: 0.0, // Monochromatic OLED energy saving
                blur_radius_px: 0.0,
                clock_scale: 0.9,
                clock_alpha: 1.0,
                glow_intensity: 0.25,
                parallax_intensity: 0.0,
                morph_progress: 0.0,
            },
            DisplayState::LockScreen => Self {
                brightness: 1.0,
                saturation: 1.0,
                blur_radius_px: 0.0,
                clock_scale: 1.0,
                clock_alpha: 1.0,
                glow_intensity: 0.50,
                parallax_intensity: 1.0,
                morph_progress: 1.0,
            },
            DisplayState::HomeScreen => Self {
                brightness: 1.0,
                saturation: 1.0,
                blur_radius_px: 24.0, // Dynamic Gaussian blur for launcher legibility
                clock_scale: 1.2,
                clock_alpha: 0.0,
                glow_intensity: 0.10,
                parallax_intensity: 0.4,
                morph_progress: 1.0,
            },
        }
    }

    pub fn interpolate(from: &Self, to: &Self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Self {
            brightness: from.brightness + (to.brightness - from.brightness) * f,
            saturation: from.saturation + (to.saturation - from.saturation) * f,
            blur_radius_px: from.blur_radius_px + (to.blur_radius_px - from.blur_radius_px) * f,
            clock_scale: from.clock_scale + (to.clock_scale - from.clock_scale) * f,
            clock_alpha: from.clock_alpha + (to.clock_alpha - from.clock_alpha) * f,
            glow_intensity: from.glow_intensity + (to.glow_intensity - from.glow_intensity) * f,
            parallax_intensity: from.parallax_intensity
                + (to.parallax_intensity - from.parallax_intensity) * f,
            morph_progress: from.morph_progress + (to.morph_progress - from.morph_progress) * f,
        }
    }
}

pub struct StateMachine {
    pub current_state: DisplayState,
    pub active_transition: Option<DisplayTransition>,
    pub time_in_current_state_ms: u32,
    pub aod_timeout_ms: Option<u32>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self {
            current_state: DisplayState::ScreenOff,
            active_transition: None,
            time_in_current_state_ms: 0,
            aod_timeout_ms: None,
        }
    }
}

impl StateMachine {
    pub fn new(initial_state: DisplayState) -> Self {
        Self {
            current_state: initial_state,
            active_transition: None,
            time_in_current_state_ms: 0,
            aod_timeout_ms: None,
        }
    }

    pub fn set_aod_timeout_seconds(&mut self, timeout_secs: Option<u32>) {
        self.aod_timeout_ms = timeout_secs.map(|s| s * 1000);
    }

    pub fn handle_event(&mut self, event: ScreenEvent) -> Option<DisplayState> {
        let next = match (self.current_state, event) {
            // Screen Off triggers
            (_, ScreenEvent::ScreenTurnedOff) => DisplayState::ScreenOff,

            // Screen Off -> Display Preview or Lock Screen
            (DisplayState::ScreenOff, ScreenEvent::ProximityDetected)
            | (DisplayState::ScreenOff, ScreenEvent::ScreenTapped) => DisplayState::DisplayPreview,
            (DisplayState::ScreenOff, ScreenEvent::PowerButtonPressed) => DisplayState::LockScreen,

            // Display Preview -> Lock Screen or Screen Off
            (DisplayState::DisplayPreview, ScreenEvent::ScreenTapped)
            | (DisplayState::DisplayPreview, ScreenEvent::PowerButtonPressed) => {
                DisplayState::LockScreen
            }
            (DisplayState::DisplayPreview, ScreenEvent::TimeoutExpired) => DisplayState::ScreenOff,

            // Lock Screen -> Home Screen, Display Preview, or Screen Off
            (DisplayState::LockScreen, ScreenEvent::BiometricSuccess)
            | (DisplayState::LockScreen, ScreenEvent::KeyguardDismissed) => {
                DisplayState::HomeScreen
            }
            (DisplayState::LockScreen, ScreenEvent::TimeoutExpired) => DisplayState::DisplayPreview,
            (DisplayState::LockScreen, ScreenEvent::PowerButtonPressed) => DisplayState::ScreenOff,

            // Home Screen -> Lock Screen or Screen Off ("vice versa")
            (DisplayState::HomeScreen, ScreenEvent::LockRequested) => DisplayState::LockScreen,
            (DisplayState::HomeScreen, ScreenEvent::PowerButtonPressed) => DisplayState::ScreenOff,

            _ => self.current_state,
        };

        if next != self.current_state {
            let transition_duration = match (self.current_state, next) {
                (DisplayState::DisplayPreview, DisplayState::LockScreen) => 350,
                (DisplayState::LockScreen, DisplayState::HomeScreen) => 400,
                (DisplayState::HomeScreen, DisplayState::LockScreen) => 300,
                (DisplayState::LockScreen, DisplayState::DisplayPreview) => 300,
                (_, DisplayState::ScreenOff) => 150,
                _ => 250,
            };

            let style = match (self.current_state, next) {
                (DisplayState::ScreenOff, DisplayState::DisplayPreview) => TransitionStyle::Fade,
                (DisplayState::DisplayPreview, DisplayState::LockScreen) => TransitionStyle::Morph,
                (DisplayState::LockScreen, DisplayState::HomeScreen) => TransitionStyle::SlideUp,
                (DisplayState::HomeScreen, DisplayState::LockScreen) => TransitionStyle::Fade,
                _ => TransitionStyle::Dissolve,
            };

            self.active_transition = Some(DisplayTransition::new(
                self.current_state,
                next,
                transition_duration,
                style,
                TransitionEasing::SmoothStep,
            ));
            self.current_state = next;
            self.time_in_current_state_ms = 0;
            Some(next)
        } else {
            None
        }
    }

    /// Advances the animation tick by delta_ms.
    /// Returns the active transition uniforms and an optional event (e.g. if AOD timed out).
    pub fn tick(&mut self, delta_ms: u32) -> (TransitionUniforms, Option<ScreenEvent>) {
        self.time_in_current_state_ms = self.time_in_current_state_ms.saturating_add(delta_ms);

        let mut triggered_event = None;

        // Check AOD timeout if active
        if self.current_state == DisplayState::DisplayPreview && self.active_transition.is_none() {
            if let Some(limit_ms) = self.aod_timeout_ms {
                if self.time_in_current_state_ms >= limit_ms {
                    triggered_event = Some(ScreenEvent::TimeoutExpired);
                }
            }
        }

        let uniforms = if let Some(ref mut transition) = self.active_transition {
            let finished = transition.advance(delta_ms);
            let factor = transition.eased_progress();
            let u_from = TransitionUniforms::for_state(transition.from);
            let u_to = TransitionUniforms::for_state(transition.to);
            let result = TransitionUniforms::interpolate(&u_from, &u_to, factor);
            if finished {
                self.active_transition = None;
            }
            result
        } else {
            TransitionUniforms::for_state(self.current_state)
        };

        (uniforms, triggered_event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_flow() {
        let mut sm = StateMachine::default();
        assert_eq!(sm.current_state, DisplayState::ScreenOff);

        let next = sm.handle_event(ScreenEvent::ScreenTapped);
        assert_eq!(next, Some(DisplayState::DisplayPreview));
        assert_eq!(sm.current_state, DisplayState::DisplayPreview);

        let next = sm.handle_event(ScreenEvent::ScreenTapped);
        assert_eq!(next, Some(DisplayState::LockScreen));

        let next = sm.handle_event(ScreenEvent::BiometricSuccess);
        assert_eq!(next, Some(DisplayState::HomeScreen));

        // Vice versa: LockRequested from HomeScreen -> LockScreen
        let next = sm.handle_event(ScreenEvent::LockRequested);
        assert_eq!(next, Some(DisplayState::LockScreen));

        // LockScreen timeout -> DisplayPreview
        let next = sm.handle_event(ScreenEvent::TimeoutExpired);
        assert_eq!(next, Some(DisplayState::DisplayPreview));
    }

    #[test]
    fn test_aod_timeout_tick() {
        let mut sm = StateMachine::new(DisplayState::DisplayPreview);
        sm.set_aod_timeout_seconds(Some(5)); // 5 second timeout

        // Tick 4000ms: no timeout yet
        let (_, ev) = sm.tick(4000);
        assert!(ev.is_none());

        // Tick another 1500ms (total 5500ms >= 5000ms): timeout should trigger
        let (_, ev) = sm.tick(1500);
        assert_eq!(ev, Some(ScreenEvent::TimeoutExpired));

        // Handle the event
        let next = sm.handle_event(ScreenEvent::TimeoutExpired);
        assert_eq!(next, Some(DisplayState::ScreenOff));
    }

    #[test]
    fn test_transition_interpolation() {
        let mut sm = StateMachine::new(DisplayState::DisplayPreview);
        sm.handle_event(ScreenEvent::ScreenTapped); // To LockScreen (350ms)
        assert!(sm.active_transition.is_some());

        // At 0ms
        let (u0, _) = sm.tick(0);
        assert!((u0.brightness - 0.15).abs() < 0.05);

        // Advance 175ms (halfway)
        let (u_mid, _) = sm.tick(175);
        assert!(u_mid.brightness > 0.15 && u_mid.brightness < 1.0);

        // Advance remainder
        let (u_end, _) = sm.tick(200);
        assert!((u_end.brightness - 1.0).abs() < 0.001);
        assert!(sm.active_transition.is_none());
    }
}
