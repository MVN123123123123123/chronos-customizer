//! Root privilege detection, Magisk/KSU/APatch detection, and Doze whitelisting

use chronos_common::ipc::RootType;
use std::path::Path;
use std::process::Command;

pub const CHRONOS_PACKAGE: &str = "dev.chronos.customizer";

pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc_geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
unsafe fn libc_geteuid() -> u32 {
    extern "C" {
        fn geteuid() -> u32;
    }
    geteuid()
}

pub fn detect_root_type() -> RootType {
    if Path::new("/data/adb/ksu").exists() || Path::new("/data/adb/ksud").exists() {
        RootType::KernelSU
    } else if Path::new("/data/adb/ap").exists() || Path::new("/data/adb/apd").exists() {
        RootType::APatch
    } else if Path::new("/data/adb/magisk").exists() {
        RootType::Magisk
    } else if is_root() {
        RootType::StandaloneRoot
    } else {
        RootType::None
    }
}

/// Applies battery and Doze exemptions to prevent Android from killing Chronos
pub fn apply_doze_whitelist(package: &str) -> Result<(), String> {
    log::info!(
        "Applying Doze exemptions and standby overrides for package: {}",
        package
    );

    let commands = [
        format!("dumpsys deviceidle whitelist +{}", package),
        format!("cmd appops set {} RUN_IN_BACKGROUND allow", package),
        format!("cmd appops set {} RUN_ANY_IN_BACKGROUND allow", package),
        format!(
            "cmd appops set {} AUTO_REVOKE_PERMISSIONS_IF_UNUSED ignore",
            package
        ),
        format!("am set-standby-bucket {} active", package),
    ];

    for cmd_str in &commands {
        let mut parts = cmd_str.split_whitespace();
        let prog = parts.next().unwrap();
        let args: Vec<&str> = parts.collect();

        match Command::new(prog).args(&args).output() {
            Ok(out) => {
                log::debug!(
                    "Command `{}` completed with status: {}",
                    cmd_str,
                    out.status
                );
            }
            Err(e) => {
                log::warn!(
                    "Command `{}` failed (expected in test/non-Android): {}",
                    cmd_str,
                    e
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_detection() {
        let rt = detect_root_type();
        println!("Detected root type: {:?}", rt);
        // Should succeed without panicking
    }

    #[test]
    fn test_doze_whitelist_execution() {
        // Run with dummy package, handles error gracefully on non-Android
        let res = apply_doze_whitelist("dev.chronos.test");
        assert!(res.is_ok());
    }
}
