use anyhow::{Result, anyhow};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use crate::config::Host;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use portable_pty::{CommandBuilder, PtySize, PtyPair};
use std::io::{Read, Write};
use std::thread;
use std::sync::Mutex as StdMutex;
use lazy_static::lazy_static;

// Global PTY writer storage
lazy_static! {
    static ref GLOBAL_PTY_WRITER: Arc<StdMutex<Option<Box<dyn Write + Send>>>> = Arc::new(StdMutex::new(None));
}

#[derive(Clone)]
pub struct SshClient {
    pub connected: bool,
    pub connecting: bool,
    pub host: Option<Host>,
}

pub enum SshEvent {
    Connected { host: Host },
    Data(Vec<u8>),
    Error(String),
    Disconnected,
}

impl Default for SshClient {
    fn default() -> Self {
        Self {
            connected: false,
            connecting: false,
            host: None,
        }
    }
}

impl SshClient {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn connect(
        &mut self,
        host: Host,
        key_path: &str,
        event_sender: mpsc::UnboundedSender<SshEvent>,
        terminal_width: u16,
        terminal_height: u16,
    ) -> Result<()> {
        if self.connecting {
            return Err(anyhow!("Already connecting"));
        }

        info!("Starting SSH connection to {}@{}:{}", host.user, host.host, host.port);
        self.connecting = true;
        self.host = Some(host.clone());

        let host_clone = host.clone();
        let key_path = key_path.to_string();
        let sender = event_sender.clone();
        
        tokio::spawn(async move {
            match Self::establish_connection_static(
                host_clone.clone(),
                &key_path,
                terminal_width,
                terminal_height,
                sender.clone(),
            ).await {
                Ok(_) => {
                    info!("SSH connection established");
                    let _ = sender.send(SshEvent::Connected { host: host_clone });
                },
                Err(e) => {
                    error!("SSH connection failed: {}", e);
                    let _ = sender.send(SshEvent::Error(e.to_string()));
                }
            }
        });

        Ok(())
    }

    async fn establish_connection_static(
        host: Host,
        key_path: &str,
        terminal_width: u16,
        terminal_height: u16,
        sender: mpsc::UnboundedSender<SshEvent>,
    ) -> Result<()> {
        // Expand tilde in key path
        let key_path = if key_path.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            key_path.replacen('~', &home, 1)
        } else {
            key_path.to_string()
        };

        // Use portable-pty for proper PTY handling
        let pty_system = portable_pty::native_pty_system();
        let pty_size = PtySize {
            rows: terminal_height,
            cols: terminal_width,
            pixel_width: 0,
            pixel_height: 0,
        };
        
        let pty_pair = pty_system.openpty(pty_size)?;
        
        // Build SSH command
        let mut cmd = CommandBuilder::new("ssh");
        cmd.arg("-i");
        cmd.arg(&key_path);
        cmd.arg("-o");
        cmd.arg("StrictHostKeyChecking=no");
        cmd.arg("-o");
        cmd.arg("UserKnownHostsFile=/dev/null");
        cmd.arg("-o");
        cmd.arg("ServerAliveInterval=30");
        cmd.arg("-o");
        cmd.arg("ServerAliveCountMax=3");
        cmd.arg("-t"); // Force pseudo-terminal allocation
        cmd.arg(format!("{}@{}", host.user, host.host));
        cmd.arg("-p");
        cmd.arg(host.port.to_string());
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLUMNS", &terminal_width.to_string());
        cmd.env("LINES", &terminal_height.to_string());
        
        // Spawn the SSH process in the PTY
        let child = pty_pair.slave.spawn_command(cmd)?;
        info!("SSH process spawned with PID: {:?}", child.process_id());
        
        // Get the PTY master for reading/writing  
        let mut pty_reader = pty_pair.master.try_clone_reader()?;
        let pty_writer = pty_pair.master.take_writer()?;
        
        // Store the PTY writer in the global storage
        {
            let mut global_writer = GLOBAL_PTY_WRITER.lock().unwrap();
            *global_writer = Some(Box::new(pty_writer));
        }
        
        // Set up PTY output reading in a background thread
        let sender_clone = sender.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match pty_reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("PTY EOF - connection closed");
                        // Clear the global writer on disconnect
                        {
                            let mut global_writer = GLOBAL_PTY_WRITER.lock().unwrap();
                            *global_writer = None;
                        }
                        let _ = sender_clone.send(SshEvent::Disconnected);
                        break;
                    },
                    Ok(n) => {
                        let _ = sender_clone.send(SshEvent::Data(buffer[..n].to_vec()));
                    },
                    Err(e) => {
                        error!("PTY read error: {}", e);
                        // Clear the global writer on error
                        {
                            let mut global_writer = GLOBAL_PTY_WRITER.lock().unwrap();
                            *global_writer = None;
                        }
                        let _ = sender_clone.send(SshEvent::Error(format!("PTY read error: {}", e)));
                        break;
                    }
                }
            }
        });
        
        // Wait a moment for connection to establish
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        
        Ok(())
    }

    pub fn handle_event(&mut self, event: SshEvent) {
        match event {
            SshEvent::Connected { host } => {
                info!("SSH connected to {}", host.name);
                self.connected = true;
                self.connecting = false;
                self.host = Some(host);
            },
            SshEvent::Disconnected => {
                info!("SSH disconnected");
                self.connected = false;
                self.connecting = false;
                self.host = None;
            },
            SshEvent::Error(err) => {
                error!("SSH error: {}", err);
                self.connected = false;
                self.connecting = false;
            },
            SshEvent::Data(_) => {
                // Data events are handled by the terminal panel directly
            }
        }
    }

    pub async fn send_input(&self, data: &[u8]) -> Result<()> {
        if self.connected {
            let global_writer = GLOBAL_PTY_WRITER.clone();
            let data = data.to_vec();
            tokio::task::spawn_blocking(move || {
                if let Ok(mut writer_guard) = global_writer.lock() {
                    if let Some(writer) = writer_guard.as_mut() {
                        writer.write_all(&data)?;
                        writer.flush()?;
                        return Ok(());
                    }
                }
                Err(anyhow!("No PTY writer available"))
            }).await?
        } else {
            Err(anyhow!("SSH not connected"))
        }
    }

    pub async fn resize_pty(&self, _width: u16, _height: u16) -> Result<()> {
        // For the SSH command-line approach, PTY resizing is more complex
        // This would require sending SIGWINCH to the SSH process
        // For now, we'll implement a simple version
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        // Clear the global PTY writer
        {
            let mut global_writer = GLOBAL_PTY_WRITER.lock().unwrap();
            *global_writer = None;
        }
        self.connected = false;
        self.connecting = false;
        self.host = None;

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn is_connecting(&self) -> bool {
        self.connecting
    }

    pub fn get_host(&self) -> Option<&Host> {
        self.host.as_ref()
    }
}
