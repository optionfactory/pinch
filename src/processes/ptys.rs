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

/// Incrementally splits a byte stream into log lines. Lines are cut at `\n`,
/// or when the pending buffer exceeds `MAX_LINE_BYTES` (at a UTF-8 boundary
/// when possible). Trailing whitespace (including `\r`) is trimmed.
#[derive(Default)]
pub struct LineSplitter {
    pending: Vec<u8>,
}

const MAX_LINE_BYTES: usize = 8192;

impl LineSplitter {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for &byte in chunk {
            self.pending.push(byte);
            if byte != b'\n' && self.pending.len() < MAX_LINE_BYTES {
                continue;
            }
            let mut split_index = self.pending.len();
            if byte != b'\n' {
                if let Err(e) = std::str::from_utf8(&self.pending) {
                    if e.error_len().is_none() {
                        split_index = e.valid_up_to();
                    }
                }
            }
            if split_index > 0 {
                lines.push(String::from_utf8_lossy(&self.pending[..split_index]).trim_end().to_string());
                self.pending.drain(..split_index);
            }
        }
        lines
    }

    /// Returns whatever is left once the stream has ended (a last line without
    /// a trailing newline, typically a crash message).
    pub fn finish(self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }
        Some(String::from_utf8_lossy(&self.pending).trim_end().to_string())
    }
}

fn handle_log_mode(mut reader: Box<dyn Read + Send>, pane_id: usize, tx_logs: mpsc::Sender<SupervisorEvent>) {
    let mut buf = [0u8; 4096];
    let mut splitter = LineSplitter::default();
    // On Linux the PTY master reports the child's exit as `Err(EIO)`, not as
    // `Ok(0)`, so both ends of the loop are "stream finished".
    while let Ok(bytes_read) = reader.read(&mut buf) {
        if bytes_read == 0 {
            break;
        }
        for line in splitter.push(&buf[..bytes_read]) {
            let _ = tx_logs.blocking_send(SupervisorEvent::LogLine(pane_id, line));
        }
    }
    if let Some(last) = splitter.finish() {
        let _ = tx_logs.blocking_send(SupervisorEvent::LogLine(pane_id, last));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_newline_and_trims_cr() {
        let mut sp = LineSplitter::default();
        assert_eq!(sp.push(b"one\r\ntwo\n"), vec!["one", "two"]);
        assert!(sp.finish().is_none());
    }

    #[test]
    fn line_spanning_chunks_is_reassembled() {
        let mut sp = LineSplitter::default();
        assert!(sp.push(b"hel").is_empty());
        assert_eq!(sp.push(b"lo\nwor"), vec!["hello"]);
        assert_eq!(sp.push(b"ld\n"), vec!["world"]);
    }

    #[test]
    fn trailing_line_without_newline_is_flushed_on_finish() {
        let mut sp = LineSplitter::default();
        assert!(sp.push(b"panicked at main.rs:3").is_empty());
        assert_eq!(sp.finish(), Some("panicked at main.rs:3".to_string()));
    }

    #[test]
    fn oversized_line_is_cut_at_utf8_boundary() {
        let mut sp = LineSplitter::default();
        // 8191 ASCII bytes followed by the first byte of a 2-byte char: the cut
        // must land before the incomplete sequence.
        let mut data = vec![b'a'; MAX_LINE_BYTES - 1];
        data.extend_from_slice("é".as_bytes());
        let lines = sp.push(&data);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), MAX_LINE_BYTES - 1);
        assert_eq!(sp.finish(), Some("é".to_string()));
    }
}
