use crate::config::OutputFormat;
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

pub const DEFAULT_CONFIG_FILE: &str = "pinch.yaml";

#[derive(Parser, Debug)]
#[command(
    name = "pinch",
    version,
    about = "A terminal-based task manager and process supervisor.",
    override_usage = "pinch [OPTIONS] <COMMAND>",
    infer_subcommands = true
)]
pub struct Cli {
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to the configuration file",
        default_value = DEFAULT_CONFIG_FILE,
        env = "PINCH_CONFIG",
        global = true
    )]
    pub config_file: String,
    #[arg(
        short = 'o',
        long = "override",
        help = "Override a configuration variable (key:value)",
        value_parser = parse_key_val,
        global = true
    )]
    pub overrides: Vec<(String, String)>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start the TUI supervisor (default action)")]
    Tui,
    #[command(about = "Manage and interact with supervised processes")]
    Process {
        #[command(subcommand)]
        command: ProcessSubcommand,
    },
    #[command(about = "Manage Docker networks")]
    Net {
        #[command(subcommand)]
        command: NetSubcommand,
    },
    #[command(about = "Inspect Docker images referenced in the configuration")]
    Image {
        #[command(subcommand)]
        command: ImageSubcommand,
    },    
    #[command(about = "Manage configuration files")]
    Config {
        #[command(subcommand)]
        command: ConfigSubcommand,
    },
    #[command(about = "Generate shell completion scripts")]
    Completion {
        #[arg(help = "Shell to generate completions for", value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProcessSubcommand {
    #[command(about = "Run a process directly in the foreground or background")]
    Run {
        #[arg(help = "Process title to execute")]
        title: String,
        #[arg(short, long, help = "Run the process in the background")]
        background: bool,
    },
    #[command(about = "Show the command associated with a process title (or all if omitted)")]
    Show {
        #[arg(help = "Optional process title to display")]
        title: Option<String>,
        #[arg(
            short = 'f',
            long = "format",
            help = "Output format (defaults to 'raw' for a single item, 'yaml' for all items)",
            value_enum
        )]
        format: Option<OutputFormat>,
    },
    #[command(about = "List all available process titles from the configuration")]
    Ls,
}

#[derive(Subcommand, Debug)]
pub enum NetSubcommand {
    #[command(about = "List all Docker networks defined in the configuration")]
    Ls,
    #[command(about = "Show the docker network create command for a specific network (or all if omitted)")]
    Show {
        #[arg(help = "Optional target network name")]
        name: Option<String>,

        #[arg(
            short = 'f',
            long = "format",
            help = "Output format (defaults to 'raw' for a single item, 'yaml' for all items)",
            value_enum
        )]
        format: Option<OutputFormat>,
    },
    #[command(about = "Create a specific Docker network (or all if omitted)")]
    Create {
        #[arg(help = "Optional target network name")]
        name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ImageSubcommand {
    #[command(about = "List all unique Docker images used by processes in the configuration")]
    Ls {
        #[arg(
            short = 'f',
            long = "format",
            help = "Output format (defaults to 'raw')",
            value_enum
        )]
        format: Option<OutputFormat>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommand {
    #[command(about = "Generate a default configuration file in the current directory")]
    Init,
    #[command(about = "Show the parsed and variable-expanded configuration")]
    Show {
        #[arg(
            short = 'f',
            long = "format",
            help = "Output format (defaults to 'yaml')",
            value_enum
        )]
        format: Option<OutputFormat>,
    },
    #[command(about = "Inspect resolved configuration variables")]
    Var {
        #[arg(help = "Optional variable name to inspect")]
        name: Option<String>,
        #[arg(
            short = 'f',
            long = "format",
            help = "Output format (defaults to 'raw' for a single var, 'yaml' for all vars)",
            value_enum
        )]
        format: Option<OutputFormat>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    s.split_once(':')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("Invalid --override format '{}'. Expected key:value", s))
}

#[derive(Debug)]
pub enum ProcessCommand {
    Ls,
    Show {
        title: Option<String>,
        format: Option<OutputFormat>,
    },
    Run {
        title: String,
        background: bool,
    },
}

#[derive(Debug)]
pub enum NetCommand {
    Ls,
    Show {
        name: Option<String>,
        format: Option<OutputFormat>,
    },
    Create {
        name: Option<String>,
    },
}

#[derive(Debug)]
pub enum ImageCommand {
    Ls { format: Option<OutputFormat> },
}

#[derive(Debug)]
pub enum ConfigCommand {
    Init,
    Show {
        format: Option<OutputFormat>,
    },
    Var {
        name: Option<String>,
        format: Option<OutputFormat>,
    },
}

#[derive(Debug)]
pub enum CliAction {
    Tui,
    Process(ProcessCommand),
    Net(NetCommand),
    Image(ImageCommand),
    Config(ConfigCommand),
    Completion(Shell),
}

#[derive(Debug)]
pub struct ParsedCli {
    pub config_file: String,
    pub vars: HashMap<String, String>,
    pub action: CliAction,
}

pub fn parse_args() -> ParsedCli {
    let cli = Cli::parse();
    let config_file = cli.config_file;
    let vars: HashMap<String, String> = cli.overrides.into_iter().collect();

    let action = match cli.command {
        Some(Commands::Tui) | None => CliAction::Tui,
        Some(Commands::Process { command }) => {
            let cmd = match command {
                ProcessSubcommand::Ls => ProcessCommand::Ls,
                ProcessSubcommand::Show { title, format } => ProcessCommand::Show { title, format },
                ProcessSubcommand::Run { title, background } => ProcessCommand::Run { title, background },
            };
            CliAction::Process(cmd)
        }
        Some(Commands::Net { command }) => {
            let cmd = match command {
                NetSubcommand::Ls => NetCommand::Ls,
                NetSubcommand::Show { name, format } => NetCommand::Show { name, format },
                NetSubcommand::Create { name } => NetCommand::Create { name },
            };
            CliAction::Net(cmd)
        }
        Some(Commands::Image { command }) => {
            let cmd = match command {
                ImageSubcommand::Ls { format } => ImageCommand::Ls { format },
            };
            CliAction::Image(cmd)
        }        
        Some(Commands::Config { command }) => {
            let cmd = match command {
                ConfigSubcommand::Init => ConfigCommand::Init,
                ConfigSubcommand::Show { format } => ConfigCommand::Show { format },
                ConfigSubcommand::Var { name, format } => ConfigCommand::Var { name, format },
            };
            CliAction::Config(cmd)
        }
        Some(Commands::Completion { shell }) => CliAction::Completion(shell),
    };

    ParsedCli {
        config_file,
        vars,
        action,
    }
}

pub fn handle_init() {
    let default_yaml = include_str!("prototype_config.yaml");
    let path = DEFAULT_CONFIG_FILE;
    if std::path::Path::new(path).exists() {
        eprintln!("Error: '{}' already exists. We don't want to overwrite it!", path);
        std::process::exit(1);
    }
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: Failed to create default config file '{}': {}", path, e);
            std::process::exit(1);
        }
    };
    if let Err(e) = file.write_all(default_yaml.as_bytes()) {
        eprintln!("Error: Failed to write to config file '{}': {}", path, e);
        std::process::exit(1);
    }
    println!("Successfully generated '{}'.", path);
}
