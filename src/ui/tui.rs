use crate::config::PinchConfig;
use crate::supervisor::{self, Supervisor, SupervisorEvent};
use crate::ui::terminal::TerminalGuard;
use crossterm::event::EventStream;
use futures::StreamExt;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

const TERMINATION_SIGNAL: &str = "SIGINT/SIGTERM/SIGHUP received";

/// Orchestrates the TUI event loop, supervisor execution, and background task lifecycle.
pub async fn run_tui(config: PinchConfig) -> Result<(), Box<dyn std::error::Error>> {
    let (tx_ui, rx_ui) = mpsc::channel::<SupervisorEvent>(100);
    let (tx_logs, rx_logs) = mpsc::channel::<SupervisorEvent>(10_000);
    let is_running = Arc::new(AtomicBool::new(true));
    // Register watches before entering the alternate screen so warnings reach
    // the user's regular terminal (they are also echoed into the pane below).
    let (watcher_handle, watch_warnings) =
        supervisor::watchers::start_watcher(&config, tx_ui.clone(), Arc::clone(&is_running))?;
    for w in &watch_warnings {
        eprintln!("Warning: {}", w.message);
    }
    let mut guard = TerminalGuard::init()?;
    let tx_signal = tx_ui.clone();
    let signal_handle = tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let (Ok(mut sigterm), Ok(mut sighup)) = (signal(SignalKind::terminate()), signal(SignalKind::hangup())) else {
            return;
        };
        // SIGHUP is the terminal window closing: stop the processes as on quit,
        // or the ones that ignore a hangup outlive pinch.
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
            _ = sighup.recv() => {}
        }
        let _ = tx_signal
            .send(SupervisorEvent::Error(TERMINATION_SIGNAL.to_string()))
            .await;
    });
    let tx_input = tx_ui.clone();
    let input_handle = tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_millis(500));
        let mut event_stream = EventStream::new();
        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    if tx_input.send(SupervisorEvent::SupervisorTick).await.is_err() {
                        break;
                    }
                }
                Some(event_result) = event_stream.next() => {
                    match event_result {
                        Ok(raw_event) => {
                            if tx_input.send(SupervisorEvent::Input(raw_event)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx_input.send(SupervisorEvent::Error(format!("Terminal input error: {}", e))).await;
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });
    let mut supervisor = Supervisor::new(config, tx_ui, tx_logs);
    for w in watch_warnings {
        if let Some(pane) = supervisor.state.panes.iter_mut().find(|p| p.id == w.pane_id) {
            pane.add_system_log(&w.message, ratatui::style::Color::Yellow);
        }
    }
    let run_result = supervisor.run(&mut guard.terminal, rx_ui, rx_logs).await;
    // signal tasks to stop
    is_running.store(false, Ordering::SeqCst);
    input_handle.abort();
    signal_handle.abort();
    // terminate processes before restoring terminal
    let shutdown_handles = supervisor.shutdown();

    //restore terminal so user sees standard stdout/stderr
    drop(guard);
    // inform user on standard stdout while child processes drain
    if !shutdown_handles.is_empty() {
        // Not println!: after a SIGHUP the terminal is gone, and its panic would
        // abandon the processes still terminating.
        let _ = writeln!(std::io::stdout(), "Pinch: Waiting for child processes to terminate...");
        for handle in shutdown_handles {
            let _ = handle.await;
        }
    }
    let _ = watcher_handle.await;
    if let Err(err) = run_result
        && err != TERMINATION_SIGNAL
    {
        eprintln!("\n[Pinch Error]: {}", err);
        std::process::exit(1);
    }
    Ok(())
}
