//! BiometricPrompt & KeyguardUpdateMonitor Callback Listener

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiometricSource {
    Fingerprint,
    Face,
    Iris,
}

#[derive(Debug, Clone)]
pub struct BiometricAuthEvent {
    pub source: BiometricSource,
    pub user_id: i32,
    pub success: bool,
    pub error_msg: Option<String>,
}

#[derive(Default)]
pub struct BiometricHookEngine {
    pub last_event: Option<BiometricAuthEvent>,
}

impl BiometricHookEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_authenticated(&mut self, user_id: i32, source: BiometricSource, is_strong: bool) {
        log::info!(
            "Biometric Hook: Authenticated successfully! Source={:?}, strong={}",
            source,
            is_strong
        );
        self.last_event = Some(BiometricAuthEvent {
            source,
            user_id,
            success: true,
            error_msg: None,
        });
    }

    pub fn on_auth_failed(&mut self, source: BiometricSource) {
        log::warn!("Biometric Hook: Auth failed! Source={:?}", source);
        self.last_event = Some(BiometricAuthEvent {
            source,
            user_id: 0,
            success: false,
            error_msg: Some("Authentication failed".to_string()),
        });
    }

    pub fn on_error(&mut self, source: BiometricSource, code: i32, msg: &str) {
        log::warn!("Biometric Hook: Auth error code {}: {}", code, msg);
        self.last_event = Some(BiometricAuthEvent {
            source,
            user_id: 0,
            success: false,
            error_msg: Some(msg.to_string()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biometric_events() {
        let mut engine = BiometricHookEngine::new();
        engine.on_authenticated(0, BiometricSource::Fingerprint, true);
        assert!(engine.last_event.as_ref().unwrap().success);

        engine.on_auth_failed(BiometricSource::Face);
        assert!(!engine.last_event.as_ref().unwrap().success);
    }
}
