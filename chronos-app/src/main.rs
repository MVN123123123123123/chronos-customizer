//! Chronos Manager Application (Slint Native Rust UI)

slint::include_modules!();

use chronos_common::config::{AodMode, ChronosConfig};
use chronos_common::ipc::{IpcClient, IpcMessage};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting Chronos Manager UI...");

    let main_window = MainWindow::new()?;

    let config_state = Arc::new(Mutex::new(ChronosConfig::default()));

    // Try connecting to companion daemon
    match IpcClient::connect() {
        Ok(mut client) => {
            log::info!("Successfully connected to customd IPC daemon");
            if let Ok(IpcMessage::StatusResponse(status)) =
                client.send_request(&IpcMessage::GetStatus)
            {
                main_window.set_daemon_status_text(
                    format!(
                        "customd: Active ({:?}) - Root: {}",
                        status.root_type, status.is_root
                    )
                    .into(),
                );
            }
            if let Ok(IpcMessage::ConfigResponse(cfg)) = client.send_request(&IpcMessage::GetConfig)
            {
                *config_state.lock().unwrap() = *cfg.clone();
                main_window.set_aod_enabled(cfg.aod.enabled);
                main_window.set_aod_refresh_rate(cfg.aod.refresh_rate_hz as i32);
                main_window.set_suppress_oem_aod(cfg.aod.suppress_oem_aod);
                main_window.set_widget_glow(cfg.aod.appearance.glow_intensity);
                main_window.set_burn_in_shift_px(cfg.aod.burn_in_protection.max_shift_px as f32);

                if let Some(secs) = cfg.aod.mode.timeout_seconds() {
                    main_window.set_aod_timeout_enabled(true);
                    main_window.set_aod_timeout_seconds(secs as i32);
                } else {
                    main_window.set_aod_timeout_enabled(false);
                }

                main_window.set_lockscreen_enabled(cfg.lockscreen.enabled);
                main_window.set_sync_with_aod(cfg.lockscreen.sync_with_aod);
                main_window.set_udfps_y_percent(cfg.lockscreen.udfps_calibration.y_percent);
                main_window.set_depth_effect_enabled(cfg.wallpaper.depth_effect.enabled);
                main_window.set_depth_threshold(cfg.wallpaper.depth_effect.depth_threshold);
                main_window.set_gyro_reactivity(cfg.wallpaper.live_shader.gyro_reactivity);
            }
        }
        Err(e) => {
            log::warn!(
                "Could not connect to customd: {}. Running in standalone mode.",
                e
            );
            main_window.set_daemon_status_text("customd: Standalone (Module Not Loaded)".into());
        }
    }

    let win_weak = main_window.as_weak();
    let cfg_clone = Arc::clone(&config_state);
    main_window.on_apply_configuration(move || {
        log::info!("Apply & Sync triggered from UI");
        let win = win_weak.unwrap();
        let mut cfg = cfg_clone.lock().unwrap();

        cfg.aod.enabled = win.get_aod_enabled();
        cfg.aod.refresh_rate_hz = win.get_aod_refresh_rate() as u32;
        cfg.aod.suppress_oem_aod = win.get_suppress_oem_aod();
        cfg.aod.appearance.glow_intensity = win.get_widget_glow();
        cfg.aod.burn_in_protection.max_shift_px = win.get_burn_in_shift_px() as i32;

        if win.get_aod_timeout_enabled() {
            cfg.aod.mode = AodMode::TimeoutSeconds(win.get_aod_timeout_seconds() as u32);
        } else {
            cfg.aod.mode = AodMode::AlwaysOn;
        }

        cfg.lockscreen.enabled = win.get_lockscreen_enabled();
        cfg.lockscreen.sync_with_aod = win.get_sync_with_aod();
        cfg.lockscreen.udfps_calibration.y_percent = win.get_udfps_y_percent();
        cfg.wallpaper.depth_effect.enabled = win.get_depth_effect_enabled();
        cfg.wallpaper.depth_effect.depth_threshold = win.get_depth_threshold();
        cfg.wallpaper.live_shader.gyro_reactivity = win.get_gyro_reactivity();

        if let Ok(mut client) = IpcClient::connect() {
            let msg = IpcMessage::UpdateConfig(Box::new(cfg.clone()));
            if let Err(e) = client.send_request(&msg) {
                log::error!("Failed to send UpdateConfig to daemon: {}", e);
            } else {
                log::info!("Successfully synchronized configuration with daemon");
            }
        }
    });

    main_window.on_test_biometric_feedback(move || {
        log::info!("Testing biometric feedback glow animation");
        if let Ok(mut client) = IpcClient::connect() {
            let _ = client.send_request(&IpcMessage::TriggerBiometricFeedback { success: true });
        }
    });

    main_window.run()?;
    Ok(())
}
