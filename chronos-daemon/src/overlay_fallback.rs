//! Standalone Root Overlay Daemon Fallback (when Zygisk is disabled or unavailable)

use chronos_common::state::DisplayState;

#[derive(Debug, Default)]
pub struct RootOverlayFallback {
    pub is_active: bool,
    pub current_state: DisplayState,
}

impl RootOverlayFallback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_display_state(&mut self, state: DisplayState) {
        self.current_state = state;
        match state {
            DisplayState::DisplayPreview => {
                log::info!("Overlay Fallback: DisplayPreview (AOD) active");
                self.is_active = true;
            }
            DisplayState::LockScreen => {
                log::info!("Overlay Fallback: LockScreen overlay active");
                self.is_active = true;
            }
            DisplayState::HomeScreen | DisplayState::ScreenOff => {
                log::info!("Overlay Fallback: Hidden");
                self.is_active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_fallback_states() {
        let mut overlay = RootOverlayFallback::new();
        assert!(!overlay.is_active);

        overlay.set_display_state(DisplayState::DisplayPreview);
        assert!(overlay.is_active);

        overlay.set_display_state(DisplayState::HomeScreen);
        assert!(!overlay.is_active);
    }
}
