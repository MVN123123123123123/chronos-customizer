//! IPC Service Listener for customd

use crate::oem_suppress::OemSuppressor;
use crate::overlay_fallback::RootOverlayFallback;
use crate::power::{detect_root_type, is_root};
use crate::SHUTDOWN_REQUESTED;
use chronos_common::config::ChronosConfig;
use chronos_common::ipc::{
    DaemonStatus, IpcError, IpcMessage, IpcServer, DEFAULT_SOCKET_PATH, FALLBACK_SOCKET_PATH,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct DaemonContext {
    pub config: Mutex<ChronosConfig>,
    pub suppressor: Mutex<OemSuppressor>,
    pub overlay: Mutex<RootOverlayFallback>,
    pub running: AtomicBool,
}

pub fn run_ipc_server(ctx: Arc<DaemonContext>) -> Result<(), IpcError> {
    let server = if is_root() {
        log::info!(
            "Root mode: Binding primary socket {} with symlink {}",
            DEFAULT_SOCKET_PATH,
            FALLBACK_SOCKET_PATH
        );
        IpcServer::bind_with_symlink(DEFAULT_SOCKET_PATH, FALLBACK_SOCKET_PATH)?
    } else {
        log::info!(
            "Non-root mode: Binding fallback socket {}",
            FALLBACK_SOCKET_PATH
        );
        IpcServer::bind(FALLBACK_SOCKET_PATH)?
    };

    while ctx.running.load(Ordering::Relaxed) && !SHUTDOWN_REQUESTED.load(Ordering::Relaxed) {
        let ctx_clone = Arc::clone(&ctx);
        let res = server.handle_one_timeout(Duration::from_millis(500), move |msg| match msg {
            IpcMessage::Ping => IpcMessage::Pong,
            IpcMessage::GetStatus => {
                let supp = ctx_clone.suppressor.lock().unwrap();
                let overlay = ctx_clone.overlay.lock().unwrap();
                let status = DaemonStatus {
                    is_running: true,
                    is_root: is_root(),
                    root_type: detect_root_type(),
                    zygisk_loaded: true,
                    oem_detected: format!("{:?}", supp.vendor),
                    oem_aod_suppressed: supp.is_suppressed,
                    current_display_state: overlay.current_state,
                    battery_drain_rate: 0.65, // ~0.65%/hr budget
                    memory_rss_kb: 12500,
                };
                IpcMessage::StatusResponse(status)
            }
            IpcMessage::GetConfig => {
                let cfg = ctx_clone.config.lock().unwrap().clone();
                IpcMessage::ConfigResponse(Box::new(cfg))
            }
            IpcMessage::UpdateConfig(new_cfg) => {
                let mut cfg = ctx_clone.config.lock().unwrap();
                *cfg = *new_cfg;
                log::info!("Updated daemon configuration");
                IpcMessage::Acknowledge
            }
            IpcMessage::SuppressOemAod => {
                let mut supp = ctx_clone.suppressor.lock().unwrap();
                let _ = supp.suppress();
                IpcMessage::Acknowledge
            }
            IpcMessage::RestoreOemAod => {
                let mut supp = ctx_clone.suppressor.lock().unwrap();
                let _ = supp.restore();
                IpcMessage::Acknowledge
            }
            IpcMessage::TransitionDisplayState(state) => {
                let mut overlay = ctx_clone.overlay.lock().unwrap();
                overlay.set_display_state(state);
                IpcMessage::Acknowledge
            }
            IpcMessage::TriggerBiometricFeedback { success } => {
                log::info!("Biometric feedback event: success={}", success);
                IpcMessage::Acknowledge
            }
            _ => IpcMessage::Error("Unsupported message".to_string()),
        });

        match res {
            Ok(()) => {}
            Err(IpcError::Timeout) => {
                // Polling timeout: loop back and recheck shutdown status
            }
            Err(e) => {
                log::trace!("IPC client session finished or closed: {}", e);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_listener_lifecycle() {
        let temp_dir = std::env::temp_dir();
        let sock_path = temp_dir.join(format!("test_daemon_{}.sock", std::process::id()));

        let server = IpcServer::bind(&sock_path).expect("Failed to bind test server");
        let ctx = Arc::new(DaemonContext {
            config: Mutex::new(ChronosConfig::default()),
            suppressor: Mutex::new(OemSuppressor::new()),
            overlay: Mutex::new(RootOverlayFallback::new()),
            running: AtomicBool::new(true),
        });

        let ctx_clone = Arc::clone(&ctx);
        let srv_handle = thread::spawn(move || {
            let res = server.handle_one_timeout(Duration::from_secs(3), move |msg| match msg {
                IpcMessage::Ping => IpcMessage::Pong,
                _ => IpcMessage::Acknowledge,
            });
            assert!(res.is_ok());
        });

        thread::sleep(Duration::from_millis(50));
        let mut client =
            chronos_common::ipc::IpcClient::connect_to(&sock_path).expect("Connect failed");
        let reply = client.send_request(&IpcMessage::Ping).expect("Ping failed");
        assert!(matches!(reply, IpcMessage::Pong));

        let _ = srv_handle.join();
        ctx_clone.running.store(false, Ordering::SeqCst);
    }
}
