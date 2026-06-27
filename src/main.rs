mod app;
mod config;
mod pane;
mod process;
mod ui;
use app::{App, AppEvent};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{fs::File, io::{self, BufReader}, panic};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "pinch.yaml"
    };

    let file = File::open(config_path).unwrap_or_else(|_| {
        panic!("Failed to open configuration file: {}", config_path)
    });
    let reader = BufReader::new(file);
     
    let raw_config: config::RawAppConfig = serde_yaml::from_reader(reader).expect("Failed to parse YAML config");
    let config = raw_config.prepare();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);
    let mut app = App::new(config, tx.clone());

    let tx_clone = tx.clone();
    std::thread::spawn(move || {
        let mut last_tick = std::time::Instant::now();
        loop {
            match event::poll(std::time::Duration::from_millis(200)) {
                Ok(true) => {
                    match event::read() {
                        Ok(raw_event) => {
                            if tx_clone.blocking_send(AppEvent::Input(raw_event)).is_err() { break; }
                        }
                        Err(e) => {
                            let _ = tx_clone.blocking_send(AppEvent::Error(format!("Terminal read failure: {}", e)));
                            break;
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    let _ = tx_clone.blocking_send(AppEvent::Error(format!("Terminal poll failure: {}", e)));
                    break;
                }
            }
            if last_tick.elapsed() >= std::time::Duration::from_millis(500) {
                if tx_clone.blocking_send(AppEvent::SupervisorTick).is_err() { break; }
                last_tick = std::time::Instant::now();
            }
        }
    });

    let mut fatal_error = None;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Some(event) = rx.recv().await {
            if let AppEvent::Error(msg) = &event {
                fatal_error = Some(msg.clone());
                break;
            }            
            app.handle_event(event);            
            if app.should_quit {
                break; 
            }
        } else {
            break;
        }
    }

    for handle in app.shutdown() {
        let _ = handle.await;
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    if let Some(err) = fatal_error {
        eprintln!("\n[Pinch Fatal Error]: {}", err);
        std::process::exit(1);
    }
    Ok(())
}