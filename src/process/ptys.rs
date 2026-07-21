use crate::config::{PaneMode, ProcessConfig};
use crate::events::PinchEvent;
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use tokio::sync::mpsc;

pub struct PtyProcess {
    pub child: Box<dyn Child + Send + Sync>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

pub fn spawn_process(
    pane_id: usize,
    cfg: &ProcessConfig,
    tx_ui: mpsc::Sender<PinchEvent>,
    tx_logs: mpsc::Sender<PinchEvent>,
) -> Result<PtyProcess, String> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let tokens = &cfg.cmd;
    if tokens.is_empty() {
        return Err(format!("Process command missing for: {}", cfg.title));
    }

    let mut cmd = CommandBuilder::new(&tokens[0]);
    if tokens.len() > 1 {
        cmd.args(&tokens[1..]);
    }

    if let Some(ref target_dir) = cfg.cwd {
        cmd.cwd(target_dir);
    } else if let Ok(current_pwd) = std::env::current_dir() {
        cmd.cwd(current_pwd);
    }

    let child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => return Err(format!("Command failed to start: {}", e)),
    };
    drop(pair.slave);

    let master_reader = pair.master.try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;
    let master_writer = pair.master.take_writer()
        .map_err(|e| format!("Failed to clone PTY writer: {}", e))?;

    let mode = cfg.mode;

    tokio::task::spawn_blocking(move || {
        match mode {
            PaneMode::Log => handle_log_mode(master_reader, pane_id, tx_logs),
            PaneMode::Tui => handle_tui_mode(master_reader, pane_id, tx_logs),
        }
        let _ = tx_ui.blocking_send(PinchEvent::ProcessExit(pane_id, false));
    });

    Ok(PtyProcess {
        child,
        writer: master_writer,
        master: pair.master,
    })
}

fn handle_log_mode(mut reader: Box<dyn Read + Send>, pane_id: usize, tx_logs: mpsc::Sender<PinchEvent>) {
    let mut buf = [0u8; 4096];
    let mut line_buffer = Vec::new();

    while let Ok(bytes_read) = reader.read(&mut buf) {
        if bytes_read == 0 {
            if !line_buffer.is_empty() {
                let clean_line = String::from_utf8_lossy(&line_buffer).trim_end().to_string();
                let _ = tx_logs.blocking_send(PinchEvent::LogLine(pane_id, clean_line));
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
                let _ = tx_logs.blocking_send(PinchEvent::LogLine(pane_id, clean_line));
                line_buffer.drain(..split_index);
            }
        }
    }
}

fn handle_tui_mode(mut reader: Box<dyn Read + Send>, pane_id: usize, tx_logs: mpsc::Sender<PinchEvent>) {
    let mut buf = [0u8; 4096];

    while let Ok(bytes_read) = reader.read(&mut buf) {
        if bytes_read == 0 {
            break;
        }
        let payload = buf[..bytes_read].to_vec();
        let _ = tx_logs.blocking_send(PinchEvent::TerminalBytes(pane_id, payload));
    }
}
