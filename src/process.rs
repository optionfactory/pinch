use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem, Child};
use std::io::BufRead;
use std::io::BufReader;
use tokio::sync::mpsc;
use crate::{app::AppEvent, config::ProcessConfig};

pub fn spawn_process(
    pane_id: usize,
    cfg: &ProcessConfig,
    tx_chan: mpsc::Sender<AppEvent>,
) -> (Box<dyn Child + Send + Sync>, Box<dyn std::io::Write + Send>) {
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .expect("Failed to open PTY");

    let tokens = &cfg.cmd;
    if tokens.is_empty() {
        panic!("Process command missing for: {}", cfg.title);
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

    let child = pair.slave.spawn_command(cmd).expect("Failed to spawn process in PTY");
    drop(pair.slave); 

    let master_reader = pair.master.try_clone_reader().expect("Failed to clone PTY reader");
    let master_writer = pair.master.take_writer().expect("Failed to take PTY writer");    
    let mut reader = BufReader::new(master_reader);
    tokio::task::spawn_blocking(move || {
        let mut line = String::new();
        while let Ok(bytes_read) = reader.read_line(&mut line) {
            if bytes_read == 0 { break; }
            let clean_line = line.trim_end().to_string();
            let _ = tx_chan.blocking_send(AppEvent::LogLine(pane_id, clean_line));
            line.clear();
        }
        let _ = tx_chan.blocking_send(AppEvent::ProcessExit(pane_id));
    });
    (child, master_writer)
}