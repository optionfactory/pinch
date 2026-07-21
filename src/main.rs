mod app;
mod cli;
mod config;
mod events;
mod process;
mod state;
mod ui;

use app::App;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use events::PinchEvent;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

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

    tokio::spawn(async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }

        restore_terminal();
        eprintln!("\n[Pinch] Interrupted by signal. Restored terminal environment cleanly.");
        std::process::exit(130);
    });

    let config_path = cli::parse_args();
    let file = File::open(&config_path).unwrap_or_else(|_| panic!("Failed to open configuration: {}", config_path));
    let reader = BufReader::new(file);
    let raw_config: config::RawPinchConfig = serde_yaml::from_reader(reader).expect("Failed to parse YAML config");
    let config = raw_config.prepare()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx_ui, rx_ui) = mpsc::channel::<PinchEvent>(100);
    let (tx_logs, rx_logs) = mpsc::channel::<PinchEvent>(10_000);
    let is_running = Arc::new(AtomicBool::new(true));

    let watcher_handle = process::watchers::start_watcher(&config, tx_ui.clone(), Arc::clone(&is_running))?;

    let tx_input = tx_ui.clone();
    let is_running_input = Arc::clone(&is_running);
    let input_handle = std::thread::spawn(move || {
        let mut last_tick = std::time::Instant::now();

        while is_running_input.load(Ordering::SeqCst) {
            match event::poll(std::time::Duration::from_millis(200)) {
                Ok(true) => match event::read() {
                    Ok(raw_event) => {
                        if tx_input.blocking_send(PinchEvent::Input(raw_event)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx_input.blocking_send(PinchEvent::Error(format!("Read fail: {}", e)));
                        break;
                    }
                },
                Ok(false) => {}
                Err(e) => {
                    let _ = tx_input.blocking_send(PinchEvent::Error(format!("Poll fail: {}", e)));
                    break;
                }
            }
            if last_tick.elapsed() >= std::time::Duration::from_millis(500) {
                if tx_input.blocking_send(PinchEvent::SupervisorTick).is_err() {
                    break;
                }
                last_tick = std::time::Instant::now();
            }
        }
    });

    let mut app = App::new(config, tx_ui, tx_logs);
    let run_result = app.run(&mut terminal, rx_ui, rx_logs).await;

    is_running.store(false, Ordering::SeqCst);
    restore_terminal();

    let _ = watcher_handle.await;
    let _ = input_handle.join();

    if let Err(err) = run_result {
        eprintln!("\n[Pinch Fatal Error]: {}", err);
        std::process::exit(1);
    }

    Ok(())
}
