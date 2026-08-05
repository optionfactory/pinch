use crate::config::{PaneMode, ProcessConfig};
use crate::supervisor::SupervisorEvent;
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use tokio::sync::mpsc;

pub struct PtyHandle {
    pub child: Box<dyn Child + Send + Sync>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

fn build_pty_command(cfg: &ProcessConfig) -> Result<CommandBuilder, String> {
    if cfg.cmd.is_empty() {
        return Err(format!("Process command missing for: {}", cfg.name));
    }
    let mut cmd = CommandBuilder::new(&cfg.cmd[0]);
    if cfg.cmd.len() > 1 {
        cmd.args(&cfg.cmd[1..]);
    }
    if let Some(ref cwd) = cfg.cwd {
        cmd.cwd(cwd);
    } else if let Ok(current_pwd) = std::env::current_dir() {
        cmd.cwd(current_pwd);
    }
    Ok(cmd)
}

pub fn spawn_process(
    pane_id: usize,
    cfg: &ProcessConfig,
    tx_logs: mpsc::Sender<SupervisorEvent>,
) -> Result<PtyHandle, String> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;
    let cmd = build_pty_command(cfg)?;
    let child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => return Err(format!("Command failed to start: {}", e)),
    };
    drop(pair.slave);
    let master_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;
    let master_writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to clone PTY writer: {}", e))?;
    let mode = cfg.mode;
    tokio::task::spawn_blocking(move || match mode {
        PaneMode::Log => handle_log_mode(master_reader, pane_id, tx_logs),
        PaneMode::Tui => handle_tui_mode(master_reader, pane_id, tx_logs),
    });
    Ok(PtyHandle {
        child,
        writer: master_writer,
        master: pair.master,
    })
}

fn handle_log_mode(mut reader: Box<dyn Read + Send>, pane_id: usize, tx_logs: mpsc::Sender<SupervisorEvent>) {
    let mut buf = [0u8; 4096];
    let mut line_buffer = Vec::new();
    while let Ok(bytes_read) = reader.read(&mut buf) {
        if bytes_read == 0 {
            if !line_buffer.is_empty() {
                let clean_line = String::from_utf8_lossy(&line_buffer).trim_end().to_string();
                let _ = tx_logs.blocking_send(SupervisorEvent::LogLine(pane_id, clean_line));
            }
            break;
        }
        for &byte in &buf[..bytes_read] {
            line_buffer.push(byte);
            if byte != b'\n' && line_buffer.len() < 8192 {
                continue;
            }
            let mut split_index = line_buffer.len();
            if byte != b'\n' {
                if let Err(e) = std::str::from_utf8(&line_buffer) {
                    if e.error_len().is_none() {
                        split_index = e.valid_up_to();
                    }
                }
            }
            if split_index > 0 {
                let clean_line = String::from_utf8_lossy(&line_buffer[..split_index])
                    .trim_end()
                    .to_string();
                let _ = tx_logs.blocking_send(SupervisorEvent::LogLine(pane_id, clean_line));
                line_buffer.drain(..split_index);
            }
        }
    }
}

fn handle_tui_mode(mut reader: Box<dyn Read + Send>, pane_id: usize, tx_logs: mpsc::Sender<SupervisorEvent>) {
    let mut buf = [0u8; 4096];
    while let Ok(bytes_read) = reader.read(&mut buf) {
        if bytes_read == 0 {
            break;
        }
        let payload = buf[..bytes_read].to_vec();
        let _ = tx_logs.blocking_send(SupervisorEvent::TerminalBytes(pane_id, payload));
    }
}
