//! chronos-ctl: Command Line Control & Diagnostics for Chronos Suite

use chronos_common::ipc::{IpcClient, IpcMessage};
use chronos_common::state::DisplayState;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    let mut client = IpcClient::connect()?;

    match command {
        "ping" => {
            let resp = client.send_request(&IpcMessage::Ping)?;
            println!("Response: {:?}", resp);
        }
        "status" => {
            let resp = client.send_request(&IpcMessage::GetStatus)?;
            if let IpcMessage::StatusResponse(status) = resp {
                println!("=== Chronos Daemon Status ===");
                println!("Running:             {}", status.is_running);
                println!("Root Privileges:     {}", status.is_root);
                println!("Root Manager:        {:?}", status.root_type);
                println!("Zygisk Loaded:       {}", status.zygisk_loaded);
                println!("OEM Detected:        {}", status.oem_detected);
                println!("OEM AOD Suppressed:  {}", status.oem_aod_suppressed);
                println!("Current State:       {:?}", status.current_display_state);
                println!("Resident Memory:     {} KB", status.memory_rss_kb);
            } else {
                eprintln!("Unexpected response: {:?}", resp);
            }
        }
        "get-config" => {
            let resp = client.send_request(&IpcMessage::GetConfig)?;
            if let IpcMessage::ConfigResponse(cfg) = resp {
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            } else {
                eprintln!("Unexpected response: {:?}", resp);
            }
        }
        "set-state" => {
            let state_str = args.get(2).map(|s| s.as_str()).unwrap_or("preview");
            let state = match state_str {
                "off" => DisplayState::ScreenOff,
                "preview" | "aod" => DisplayState::DisplayPreview,
                "lock" | "lockscreen" => DisplayState::LockScreen,
                "home" | "homescreen" => DisplayState::HomeScreen,
                _ => {
                    eprintln!("Unknown state: {}. Use off|preview|lock|home", state_str);
                    std::process::exit(1);
                }
            };
            let resp = client.send_request(&IpcMessage::TransitionDisplayState(state))?;
            println!("Transition response: {:?}", resp);
        }
        "suppress-oem" => {
            let resp = client.send_request(&IpcMessage::SuppressOemAod)?;
            println!("Suppress OEM AOD response: {:?}", resp);
        }
        "restore-oem" => {
            let resp = client.send_request(&IpcMessage::RestoreOemAod)?;
            println!("Restore OEM AOD response: {:?}", resp);
        }
        "biometric-feedback" => {
            let success = args.get(2).map(|s| s.as_str() != "fail").unwrap_or(true);
            let resp = client.send_request(&IpcMessage::TriggerBiometricFeedback { success })?;
            println!("Biometric feedback response: {:?}", resp);
        }
        "help" | "--help" | "-h" => {
            println!("Usage: chronos-ctl <command> [args]");
            println!("Commands:");
            println!("  ping                     Test IPC roundtrip latency");
            println!("  status                   Query daemon runtime status & root info");
            println!("  get-config               Query active JSON configuration");
            println!(
                "  set-state <state>        Trigger display transition (off|preview|lock|home)"
            );
            println!("  suppress-oem             Suppress stock OEM AOD");
            println!("  restore-oem              Restore stock OEM AOD");
            println!("  biometric-feedback [ok|fail] Trigger simulated biometric scan pulse");
        }
        _ => {
            eprintln!(
                "Unknown command: {}. Run 'chronos-ctl help' for usage.",
                command
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
