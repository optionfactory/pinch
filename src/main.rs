mod cli;
mod config;
mod networks;
mod process;
mod runners;
mod supervisor;
mod ui;
mod vars;

use crate::config::RunMode;
use crossterm::event::EventStream;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use supervisor::Supervisor;
use supervisor::SupervisorEvent;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

fn restore_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, DisableMouseCapture);
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(20) {
        //prevent crossterm mouse events to spam the console with garbage
        if let Ok(true) = event::poll(std::time::Duration::from_millis(5)) {
            let _ = event::read();
        }
    }
    let _ = execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show);
    let _ = disable_raw_mode();
    let _ = stdout.flush();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
    let parsed = cli::parse_args();
    match &parsed.action {
        cli::CliAction::Completion(shell) => {
            use clap::CommandFactory;
            use clap_complete::{Shell as ClapShell, generate};
            let mut cmd = cli::Cli::command();
            let target_shell = match shell {
                cli::Shell::Bash => ClapShell::Bash,
                cli::Shell::Zsh => ClapShell::Zsh,
                cli::Shell::Fish => ClapShell::Fish,
            };
            generate(target_shell, &mut cmd, "pinch", &mut std::io::stdout());
            return Ok(());
        }
        cli::CliAction::Config(cli::ConfigCommand::Init) => {
            cli::handle_init();
            return Ok(());
        }
        _ => {}
    }
    let config_path = &parsed.config_file;
    let file = File::open(config_path).map_err(|e| format!("Failed to open configuration '{}': {}", config_path, e))?;
    let reader = BufReader::new(file);
    let raw_config: config::PinchManifest =
        serde_yaml::from_reader(reader).map_err(|e| format!("Failed to parse YAML config: {}", e))?;
    match parsed.action {
        cli::CliAction::Project(cmd) => match cmd {
            cli::ProjectCommand::Show { format } => {
                cli::show_project(&raw_config, format)?;
                return Ok(());
            }
        },
        cli::CliAction::Config(cmd) => match cmd {
            cli::ConfigCommand::Show { format } => {
                cli::show_config(&raw_config, &parsed.vars, format)?;
                return Ok(());
            }
            cli::ConfigCommand::Var { name, format } => {
                cli::show_vars(&raw_config, &parsed.vars, name.as_deref(), format)?;
                return Ok(());
            }
            cli::ConfigCommand::Init => unreachable!(),
        },
        cli::CliAction::Process(cmd) => match cmd {
            cli::ProcessCommand::Ls => {
                println!("Available process titles in '{}':", config_path);
                if let Some(processes) = raw_config.processes {
                    for proc in processes {
                        println!("  - {}", proc.title);
                    }
                } else {
                    println!("  (No processes defined)");
                }
                return Ok(());
            }
            cli::ProcessCommand::Show { title, format } => {
                cli::show_processes(&raw_config, &parsed.vars, title.as_deref(), format)?;
                return Ok(());
            }
            cli::ProcessCommand::Run { title, background } => {
                let config = raw_config.prepare(parsed.vars.clone(), background)?;
                networks::create_networks(&raw_config, &parsed.vars, None)?;
                let proc = config
                    .processes
                    .iter()
                    .find(|p| p.title == title)
                    .ok_or_else(|| format!("Process with title '{}' not found in configuration", title))?;
                let mut cmd = process::build_std_command(proc)?;
                if proc.run_mode == RunMode::Spawn {
                    cmd.stdin(std::process::Stdio::null());
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());
                    match cmd.spawn() {
                        Ok(child) => {
                            println!("Started '{}' in the background (PID: {})", title, child.id());
                            return Ok(());
                        }
                        Err(e) => return Err(format!("Failed to spawn background process: {}", e).into()),
                    }
                } else {
                    let err = cmd.exec();
                    return Err(format!("Failed to execute command: {}", err).into());
                }
            }
        },
        cli::CliAction::Net(cmd) => match cmd {
            cli::NetCommand::Ls => {
                println!("Available Docker networks in '{}':", config_path);
                cli::list_networks(&raw_config);
                return Ok(());
            }
            cli::NetCommand::Show { name, format } => {
                cli::show_networks(&raw_config, &parsed.vars, name.as_deref(), format)?;
                return Ok(());
            }
            cli::NetCommand::Create { name } => {
                networks::create_networks(&raw_config, &parsed.vars, name.as_deref())?;
                if let Some(n) = name {
                    println!("Successfully created Docker network '{}'.", n);
                } else {
                    println!("Successfully created all defined Docker networks.");
                }
                return Ok(());
            }
        },
        cli::CliAction::Image(cmd) => match cmd {
            cli::ImageCommand::Ls { format } => {
                cli::list_images(&raw_config, &parsed.vars, format)?;
                return Ok(());
            }
        },
        cli::CliAction::Tui => {
            let config = raw_config.prepare(parsed.vars.clone(), false)?;
            networks::create_networks(&raw_config, &parsed.vars, None)?;
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            rt.block_on(async {
                enable_raw_mode()?;
                let mut stdout = io::stdout();
                execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
                let backend = CrosstermBackend::new(stdout);
                let mut terminal = Terminal::new(backend)?;
                let (tx_ui, rx_ui) = mpsc::channel::<SupervisorEvent>(100);
                let (tx_logs, rx_logs) = mpsc::channel::<SupervisorEvent>(10_000);
                let is_running = Arc::new(AtomicBool::new(true));
                let tx_signal = tx_ui.clone();
                let signal_handle = tokio::spawn(async move {
                    use tokio::signal::unix::{SignalKind, signal};
                    let mut sigterm = match signal(SignalKind::terminate()) {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {}
                        _ = sigterm.recv() => {}
                    }
                    let _ = tx_signal.send(SupervisorEvent::Error("SIGINT/SIGTERM received".to_string())).await;
                });
                let watcher_handle = supervisor::watchers::start_watcher(&config, tx_ui.clone(), Arc::clone(&is_running))?;
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
                let run_result = supervisor.run(&mut terminal, rx_ui, rx_logs).await;
                // signal tasks to stop
                is_running.store(false, Ordering::SeqCst);
                input_handle.abort();
                signal_handle.abort();
                // terminate processes before restoring terminal
                let shutdown_handles = supervisor.shutdown();
                //restore terminal so user sees standard stdout/stderr
                restore_terminal();
                // inform user on standard stdout while child processes drain
                if !shutdown_handles.is_empty() {
                    println!("Pinch: Waiting for child processes to terminate...");
                    for handle in shutdown_handles {
                        let _ = handle.await;
                    }
                }
                let _ = watcher_handle.await;
                if let Err(err) = run_result {
                    if err != "SIGINT/SIGTERM received" {
                        eprintln!("\n[Pinch Error]: {}", err);
                        std::process::exit(1);
                    }
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            })?;
        }
        cli::CliAction::Completion(_) => unreachable!(),
    }
    Ok(())
}
