mod cli;
mod config;
mod networks;
mod process;
mod runners;
mod supervisor;
mod ui;
mod vars;

use crate::config::RunMode;
use std::fs::File;
use std::io::BufReader;
use std::os::unix::process::CommandExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        cli::CliAction::Audit(cmd) => match cmd {
            cli::AuditCommand::Show { format } => {
                cli::show_audit(&raw_config, &parsed.vars, format)?;
                return Ok(());
            }
        },
        cli::CliAction::Process(cmd) => match cmd {
            cli::ProcessCommand::Ls { format } => {
                cli::list_processes(&raw_config, format)?;
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
        cli::CliAction::Networks(cmd) => match cmd {
            cli::NetCommand::Ls { format } => {
                cli::list_networks(&raw_config, format)?;
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
        cli::CliAction::Containers(cmd) => match cmd {
            cli::ContainerCommand::Ls { format } => {
                cli::list_images(&raw_config, &parsed.vars, format)?;
                return Ok(());
            }
        },
        cli::CliAction::Tui => {
            let config = raw_config.prepare(parsed.vars.clone(), false)?;
            networks::create_networks(&raw_config, &parsed.vars, None)?;
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            rt.block_on(ui::tui::run_tui(config))?;
        }
        cli::CliAction::Completion(_) => unreachable!(),
    }
    Ok(())
}