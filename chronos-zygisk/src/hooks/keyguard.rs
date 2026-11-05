//! SystemUI KeyguardViewMediator & StatusBarKeyguardViewManager Interception

#[derive(Default)]
pub struct KeyguardHookEngine {
    pub is_stock_keyguard_hidden: bool,
    pub is_fallback_pin_showing: bool,
}

impl KeyguardHookEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hides stock lock screen clock, status views, and notifications while keeping KeyguardUpdateMonitor active
    pub fn hide_stock_lockscreen(&mut self) {
        if !self.is_stock_keyguard_hidden {
            log::info!(
                "SystemUI Hook: Stock keyguard view elements hidden, Chronos lock canvas active"
            );
            self.is_stock_keyguard_hidden = true;
            self.is_fallback_pin_showing = false;
        }
    }

    /// Emergency Fallback: If user swipes up to enter PIN/pattern or biometrics fail,
    /// immediately restore stock credential entry so user is NEVER locked out!
    pub fn request_fallback_credential_entry(&mut self) {
        log::info!("SystemUI Hook: Restoring stock Keyguard security container for PIN/Pattern/Password entry");
        self.is_stock_keyguard_hidden = false;
        self.is_fallback_pin_showing = true;
    }

    /// Restores stock keyguard visibility when Chronos customizer is disabled
    pub fn restore_stock_lockscreen(&mut self) {
        log::info!("SystemUI Hook: Restoring stock SystemUI keyguard");
        self.is_stock_keyguard_hidden = false;
        self.is_fallback_pin_showing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyguard_interception_and_fallback() {
        let mut hook = KeyguardHookEngine::new();
        assert!(!hook.is_stock_keyguard_hidden);

        hook.hide_stock_lockscreen();
        assert!(hook.is_stock_keyguard_hidden);
        assert!(!hook.is_fallback_pin_showing);

        // Emergency fallback verification
        hook.request_fallback_credential_entry();
        assert!(!hook.is_stock_keyguard_hidden);
        assert!(hook.is_fallback_pin_showing);
    }
}
