//! customd: Native Root Companion Daemon for Chronos Customizer Suite

pub mod oem_suppress;
pub mod overlay_fallback;
pub mod power;
pub mod service_listener;

use chronos_common::config::ChronosConfig;
use oem_suppress::OemSuppressor;
use overlay_fallback::RootOverlayFallback;
use power::{apply_doze_whitelist, detect_root_type, is_root, CHRONOS_PACKAGE};
use service_listener::{run_ipc_server, DaemonContext};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting Chronos Companion Root Daemon (customd)...");

    let root = is_root();
    let root_type = detect_root_type();
    log::info!(
        "Execution privileges: is_root={}, root_manager={:?}",
        root,
        root_type
    );

    // Whitelist Chronos manager package from Doze Mode
    if let Err(e) = apply_doze_whitelist(CHRONOS_PACKAGE) {
        log::warn!("Could not apply Doze whitelist: {}", e);
    }

    let ctx = Arc::new(DaemonContext {
        config: Mutex::new(ChronosConfig::default()),
        suppressor: Mutex::new(OemSuppressor::new()),
        overlay: Mutex::new(RootOverlayFallback::new()),
        running: AtomicBool::new(true),
    });

    // Safe signal handling for graceful shutdown and OEM AOD restoration
    #[cfg(unix)]
    unsafe {
        setup_signal_handlers();
    }

    log::info!("customd initialized. Listening for IPC requests...");
    if let Err(e) = run_ipc_server(Arc::clone(&ctx)) {
        log::error!("IPC Server exited with error: {}", e);
    }

    // Graceful cleanup: ensure OEM settings are restored on shutdown
    log::info!("Shutting down customd: restoring OEM settings and cleaning up...");
    if let Ok(mut supp) = ctx.suppressor.lock() {
        if supp.is_suppressed {
            let _ = supp.restore();
            log::info!("OEM AOD settings successfully restored to original system state.");
        }
    }
    log::info!("customd shutdown complete.");
}

#[cfg(unix)]
unsafe fn setup_signal_handlers() {
    extern "C" fn handle_signal(_: i32) {
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
    }
    libc_signal(2, handle_signal); // SIGINT
    libc_signal(15, handle_signal); // SIGTERM
}

#[cfg(unix)]
unsafe fn libc_signal(sig: i32, handler: extern "C" fn(i32)) {
    extern "C" {
        fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    }
    signal(sig, handler);
}
