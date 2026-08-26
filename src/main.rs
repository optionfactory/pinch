// src/main.rs
use pinch::cli::{self, CliAction, ConfigurationCommand, ContainerCommand, NetCommand, ProcessCommand, Shell};
use pinch::config::{PinchManifest, RunMode};
use pinch::networks;
use pinch::processes;
use pinch::ui;
use std::fs::File;
use std::io::BufReader;
use std::os::unix::process::CommandExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = cli::parse_args();
    match &parsed.action {
        CliAction::Completion(shell) => {
            use clap::CommandFactory;
            use clap_complete::{Shell as ClapShell, generate};
            let mut cmd = cli::Cli::command();
            let target_shell = match shell {
                Shell::Bash => ClapShell::Bash,
                Shell::Zsh => ClapShell::Zsh,
                Shell::Fish => ClapShell::Fish,
            };
            generate(target_shell, &mut cmd, "pinch", &mut std::io::stdout());
            return Ok(());
        }
        CliAction::Configuration(ConfigurationCommand::Init) => {
            cli::handle_init();
            return Ok(());
        }
        _ => {}
    }
    let config_path = &parsed.config_file;
    let file = File::open(config_path).map_err(|e| format!("Failed to open configuration '{}': {}", config_path, e))?;
    let reader = BufReader::new(file);
    let raw_config: PinchManifest =
        serde_saphyr::from_reader(reader).map_err(|e| format!("Failed to parse YAML config: {}", e))?;
    raw_config.validate_schema_version()?;
    match parsed.action {
        CliAction::Configuration(cmd) => match cmd {
            ConfigurationCommand::Show { format } => {
                cli::show_config(&raw_config, &parsed.vars, format)?;
                return Ok(());
            }
            ConfigurationCommand::Var { name, format } => {
                cli::show_vars(&raw_config, &parsed.vars, name.as_deref(), format)?;
                return Ok(());
            }
            ConfigurationCommand::Init => unreachable!(),
        },
        CliAction::Process(cmd) => match cmd {
            ProcessCommand::Ls { format } => {
                cli::list_processes(&raw_config, format)?;
                return Ok(());
            }
            ProcessCommand::Show { name, format } => {
                cli::show_processes(&raw_config, &parsed.vars, name.as_deref(), format)?;
                return Ok(());
            }
            ProcessCommand::Run { name, background } => {
                let config = raw_config.prepare(parsed.vars.clone(), background)?;
                networks::create_networks(&raw_config, &parsed.vars, None)?;

                let proc = config
                    .processes
                    .iter()
                    .find(|p| p.name == name)
                    .ok_or_else(|| format!("Process with name '{}' not found in configuration", name))?;

                let mut cmd = processes::build_std_command(proc)?;

                if proc.run_mode == RunMode::Spawn {
                    cmd.stdin(std::process::Stdio::null());
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());
                    // Detach from the shell's job: in its own process group the
                    // child no longer receives the SIGHUP/SIGINT aimed at the
                    // foreground job when the terminal closes or ^C is pressed.
                    cmd.process_group(0);
                    match cmd.spawn() {
                        Ok(child) => {
                            println!("Started '{}' in the background (PID: {})", name, child.id());
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
        CliAction::Networks(cmd) => match cmd {
            NetCommand::Ls { format } => {
                cli::list_networks(&raw_config, format)?;
                return Ok(());
            }
            NetCommand::Show { name, format } => {
                cli::show_networks(&raw_config, &parsed.vars, name.as_deref(), format)?;
                return Ok(());
            }
            NetCommand::Create { name } => {
                networks::create_networks(&raw_config, &parsed.vars, name.as_deref())?;
                if let Some(n) = name {
                    println!("Successfully created Docker network '{}'.", n);
                } else {
                    println!("Successfully created all defined Docker networks.");
                }
                return Ok(());
            }
        },
        CliAction::Containers(cmd) => match cmd {
            ContainerCommand::Ls { format } => {
                cli::list_images(&raw_config, &parsed.vars, format)?;
                return Ok(());
            }
        },
        CliAction::Tui => {
            let config = raw_config.prepare(parsed.vars.clone(), false)?;
            networks::create_networks(&raw_config, &parsed.vars, None)?;

            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            rt.block_on(ui::tui::run_tui(config))?;
        }
        CliAction::Completion(_) => unreachable!(),
    }

    Ok(())
}
