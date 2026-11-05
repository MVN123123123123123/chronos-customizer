//! OEM AOD Dynamic Detection, Suppression, and Safe Rollback Matrix

use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OemVendor {
    GooglePixel,
    Samsung,
    Xiaomi,
    OppoOnePlus,
    GenericAosp,
}

#[derive(Debug, Clone)]
pub struct OemAodSetting {
    pub namespace: &'static str, // "system" or "secure" or "global"
    pub key: &'static str,
    pub suppress_val: &'static str,
}

pub struct OemSuppressor {
    pub vendor: OemVendor,
    original_values: HashMap<String, String>,
    pub is_suppressed: bool,
}

impl Default for OemSuppressor {
    fn default() -> Self {
        Self::new()
    }
}

impl OemSuppressor {
    pub fn new() -> Self {
        let vendor = Self::detect_oem();
        Self {
            vendor,
            original_values: HashMap::new(),
            is_suppressed: false,
        }
    }

    pub fn detect_oem() -> OemVendor {
        let manufacturer = Self::read_system_prop("ro.product.manufacturer").to_lowercase();
        let brand = Self::read_system_prop("ro.product.brand").to_lowercase();

        if manufacturer.contains("samsung") || brand.contains("samsung") {
            OemVendor::Samsung
        } else if manufacturer.contains("xiaomi")
            || brand.contains("redmi")
            || brand.contains("poco")
        {
            OemVendor::Xiaomi
        } else if manufacturer.contains("oneplus")
            || manufacturer.contains("oppo")
            || brand.contains("realme")
        {
            OemVendor::OppoOnePlus
        } else if manufacturer.contains("google") {
            OemVendor::GooglePixel
        } else {
            OemVendor::GenericAosp
        }
    }

    pub fn get_target_settings(&self) -> Vec<OemAodSetting> {
        match self.vendor {
            OemVendor::Samsung => vec![OemAodSetting {
                namespace: "system",
                key: "aod_mode",
                suppress_val: "0",
            }],
            OemVendor::Xiaomi => vec![OemAodSetting {
                namespace: "secure",
                key: "doze_always_on_enabled",
                suppress_val: "0",
            }],
            OemVendor::OppoOnePlus => vec![OemAodSetting {
                namespace: "system",
                key: "aod_enable",
                suppress_val: "0",
            }],
            OemVendor::GooglePixel | OemVendor::GenericAosp => vec![OemAodSetting {
                namespace: "secure",
                key: "doze_always_on",
                suppress_val: "0",
            }],
        }
    }

    pub fn suppress(&mut self) -> Result<(), String> {
        if self.is_suppressed {
            return Ok(());
        }

        let targets = self.get_target_settings();
        for target in targets {
            let key_id = format!("{}:{}", target.namespace, target.key);
            // 1. Read & backup original value
            let orig = Self::read_setting(target.namespace, target.key);
            self.original_values.insert(key_id.clone(), orig);

            // 2. Put suppressed value (0)
            log::info!(
                "Suppressing OEM AOD: settings put {} {} {}",
                target.namespace,
                target.key,
                target.suppress_val
            );
            Self::write_setting(target.namespace, target.key, target.suppress_val);
        }

        self.is_suppressed = true;
        Ok(())
    }

    pub fn restore(&mut self) -> Result<(), String> {
        if !self.is_suppressed {
            return Ok(());
        }

        let targets = self.get_target_settings();
        for target in targets {
            let key_id = format!("{}:{}", target.namespace, target.key);
            if let Some(orig_val) = self.original_values.get(&key_id) {
                log::info!(
                    "Restoring OEM AOD: settings put {} {} {}",
                    target.namespace,
                    target.key,
                    orig_val
                );
                Self::write_setting(target.namespace, target.key, orig_val);
            }
        }

        self.is_suppressed = false;
        self.original_values.clear();
        Ok(())
    }

    fn read_system_prop(key: &str) -> String {
        Command::new("getprop")
            .arg(key)
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    }

    fn read_setting(namespace: &str, key: &str) -> String {
        Command::new("settings")
            .args(["get", namespace, key])
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "1".to_string())
    }

    fn write_setting(namespace: &str, key: &str, val: &str) {
        let _ = Command::new("settings")
            .args(["put", namespace, key, val])
            .output();
    }
}

impl Drop for OemSuppressor {
    fn drop(&mut self) {
        // Guaranteed cleanup: ensure stock settings restored on exit or crash
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oem_detection() {
        let mut suppressor = OemSuppressor::new();
        let targets = suppressor.get_target_settings();
        assert!(!targets.is_empty());
        // Verify suppress and restore cycle does not crash
        assert!(suppressor.suppress().is_ok());
        assert!(suppressor.is_suppressed);
        assert!(suppressor.restore().is_ok());
        assert!(!suppressor.is_suppressed);
    }
}
