use crate::config::ChronosConfig;
use crate::state::DisplayState;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_SOCKET_PATH: &str = "/dev/chronos.sock";
pub const FALLBACK_SOCKET_PATH: &str = "/data/local/tmp/chronos.sock";

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Timeout error")]
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootType {
    Magisk,
    KernelSU,
    APatch,
    StandaloneRoot,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub is_running: bool,
    pub is_root: bool,
    pub root_type: RootType,
    pub zygisk_loaded: bool,
    pub oem_detected: String,
    pub oem_aod_suppressed: bool,
    pub current_display_state: DisplayState,
    pub battery_drain_rate: f32,
    pub memory_rss_kb: usize,
}

impl Default for DaemonStatus {
    fn default() -> Self {
        Self {
            is_running: true,
            is_root: true,
            root_type: RootType::KernelSU,
            zygisk_loaded: true,
            oem_detected: "AOSP".to_string(),
            oem_aod_suppressed: false,
            current_display_state: DisplayState::ScreenOff,
            battery_drain_rate: 0.0,
            memory_rss_kb: 14200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    Ping,
    Pong,
    GetConfig,
    ConfigResponse(Box<ChronosConfig>),
    UpdateConfig(Box<ChronosConfig>),
    GetStatus,
    StatusResponse(DaemonStatus),
    SuppressOemAod,
    RestoreOemAod,
    TriggerBiometricFeedback { success: bool },
    TransitionDisplayState(DisplayState),
    Acknowledge,
    Error(String),
}

impl IpcMessage {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), IpcError> {
        let payload = bincode::serialize(self)?;
        let length = payload.len() as u32;
        writer.write_all(&length.to_be_bytes())?;
        writer.write_all(&payload)?;
        writer.flush()?;
        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, IpcError> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let length = u32::from_be_bytes(len_buf) as usize;

        // Hard cap on payload size (8 MB) for safety
        if length > 8 * 1024 * 1024 {
            return Err(IpcError::Protocol(
                "Payload exceeds maximum size".to_string(),
            ));
        }

        let mut payload = vec![0u8; length];
        reader.read_exact(&mut payload)?;
        let msg = bincode::deserialize(&payload)?;
        Ok(msg)
    }
}

pub struct IpcClient {
    stream: UnixStream,
}

impl IpcClient {
    pub fn connect() -> Result<Self, IpcError> {
        let stream = UnixStream::connect(DEFAULT_SOCKET_PATH)
            .or_else(|_| UnixStream::connect(FALLBACK_SOCKET_PATH))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { stream })
    }

    pub fn connect_to<P: AsRef<Path>>(path: P) -> Result<Self, IpcError> {
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { stream })
    }

    pub fn send_request(&mut self, request: &IpcMessage) -> Result<IpcMessage, IpcError> {
        request.write_to(&mut self.stream)?;
        IpcMessage::read_from(&mut self.stream)
    }
}

pub struct IpcServer {
    listener: UnixListener,
    path: PathBuf,
    symlink_path: Option<PathBuf>,
}

impl IpcServer {
    pub fn bind<P: AsRef<Path>>(path: P) -> Result<Self, IpcError> {
        let path_buf = path.as_ref().to_path_buf();
        if path_buf.exists() {
            let _ = std::fs::remove_file(&path_buf);
        }
        let listener = UnixListener::bind(&path_buf)?;
        // Set nonblocking to allow timeout-based accept checking
        listener.set_nonblocking(true)?;

        // Make socket accessible to root / system / apps
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path_buf, std::fs::Permissions::from_mode(0o666));
        }

        Ok(Self {
            listener,
            path: path_buf,
            symlink_path: None,
        })
    }

    pub fn bind_with_symlink<P: AsRef<Path>, S: AsRef<Path>>(
        path: P,
        symlink: S,
    ) -> Result<Self, IpcError> {
        let mut server = Self::bind(path)?;
        let link_buf = symlink.as_ref().to_path_buf();
        if link_buf.exists() || std::fs::symlink_metadata(&link_buf).is_ok() {
            let _ = std::fs::remove_file(&link_buf);
        }
        #[cfg(unix)]
        {
            if let Ok(()) = std::os::unix::fs::symlink(&server.path, &link_buf) {
                server.symlink_path = Some(link_buf);
            }
        }
        Ok(server)
    }

    pub fn handle_one<F>(&self, handler: F) -> Result<(), IpcError>
    where
        F: FnMut(IpcMessage) -> IpcMessage,
    {
        self.handle_one_timeout(Duration::from_secs(3600), handler)
    }

    pub fn handle_one_timeout<F>(&self, timeout: Duration, mut handler: F) -> Result<(), IpcError>
    where
        F: FnMut(IpcMessage) -> IpcMessage,
    {
        let start = std::time::Instant::now();
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false)?;
                    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
                    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
                    let msg = IpcMessage::read_from(&mut stream)?;
                    let response = handler(msg);
                    response.write_to(&mut stream)?;
                    return Ok(());
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        return Err(IpcError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(IpcError::Io(e)),
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        if let Some(ref symlink) = self.symlink_path {
            if symlink.exists() || std::fs::symlink_metadata(symlink).is_ok() {
                let _ = std::fs::remove_file(symlink);
            }
        }
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn test_ipc_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let socket_path = temp_dir.join(format!("test_chronos_ipc_{}.sock", std::process::id()));

        let server = IpcServer::bind(&socket_path).expect("Failed to bind server");
        let (tx, rx) = mpsc::channel();

        let srv_handle = thread::spawn(move || {
            tx.send(()).unwrap();
            server
                .handle_one_timeout(Duration::from_secs(5), |msg| match msg {
                    IpcMessage::Ping => IpcMessage::Pong,
                    IpcMessage::GetStatus => IpcMessage::StatusResponse(DaemonStatus::default()),
                    _ => IpcMessage::Error("Unknown".to_string()),
                })
                .expect("Failed to handle IPC connection");
        });

        rx.recv().unwrap(); // Wait until server is listening

        let mut client = IpcClient::connect_to(&socket_path).expect("Client failed to connect");
        let reply = client
            .send_request(&IpcMessage::Ping)
            .expect("Failed to send ping");
        match reply {
            IpcMessage::Pong => {}
            other => panic!("Unexpected reply: {:?}", other),
        }

        let _ = srv_handle.join();
        if socket_path.exists() {
            let _ = std::fs::remove_file(&socket_path);
        }
    }
}
