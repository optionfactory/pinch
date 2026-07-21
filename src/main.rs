mod app;
mod cli;
mod config;
mod events;
mod process;
mod state;
mod ui;

use app::App;
use crossterm::event::EventStream;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use events::PinchEvent;
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

fn restore_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, DisableMouseCapture);
    while let Ok(true) = event::poll(std::time::Duration::from_millis(0)) {
        let _ = event::read();
    }
    let _ = execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show);
    let _ = disable_raw_mode();
    let _ = stdout.flush();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    let config_path = cli::parse_args();
    let file =
        File::open(&config_path).map_err(|e| format!("Failed to open configuration '{}': {}", config_path, e))?;
    let reader = BufReader::new(file);
    let raw_config: config::RawPinchConfig =
        serde_yaml::from_reader(reader).map_err(|e| format!("Failed to parse YAML config: {}", e))?;
    let config = raw_config.prepare()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx_ui, rx_ui) = mpsc::channel::<PinchEvent>(100);
    let (tx_logs, rx_logs) = mpsc::channel::<PinchEvent>(10_000);
    let is_running = Arc::new(AtomicBool::new(true));

    let tx_signal = tx_ui.clone();
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                let _ = tx_signal
                    .send(PinchEvent::Error(format!("Failed to bind to SIGTERM: {}", e)))
                    .await;
                return;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        let _ = tx_signal
            .send(PinchEvent::Error("Interrupted by OS signal".to_string()))
            .await;
    });

    let watcher_handle = process::watchers::start_watcher(&config, tx_ui.clone(), Arc::clone(&is_running))?;

    let tx_input = tx_ui.clone();
    let is_running_input = Arc::clone(&is_running);
    let input_handle = tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_millis(500));
        let mut event_stream = EventStream::new();

        while is_running_input.load(Ordering::SeqCst) {
            tokio::select! {
                _ = tick_interval.tick() => {
                    if tx_input.send(PinchEvent::SupervisorTick).await.is_err() {
                        break;
                    }
                }
                Some(event_result) = event_stream.next() => {
                    match event_result {
                        Ok(raw_event) => {
                            if tx_input.send(PinchEvent::Input(raw_event)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx_input.send(PinchEvent::Error(format!("Terminal input error: {}", e))).await;
                            break;
                        }
                    }
                }
                else => {
                    break;
                }
            }
        }
    });
    let mut app = App::new(config, tx_ui, tx_logs);
    let run_result = app.run(&mut terminal, rx_ui, rx_logs).await;

    is_running.store(false, Ordering::SeqCst);
    restore_terminal();

    let _ = watcher_handle.await;
    let _ = input_handle.await;

    if let Err(err) = run_result {
        eprintln!("\n[Pinch Fatal Error]: {}", err);
        std::process::exit(1);
    }

    Ok(())
}
