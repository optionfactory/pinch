use crate::config::{DockerNetworkConfig, LayoutBlock, OutputFormat, PaneMode, PinchManifest};
use crate::runners::{RunBuilder, RunContext};
use crate::vars::apply_vars;
use crate::vars::builtin_vars;
use portable_pty::CommandBuilder;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct PinchConfig {
    pub title: String,
    pub processes: Vec<ProcessConfig>,
    pub logs_max_size: Option<usize>,
    pub layout: Vec<LayoutBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RunMode {
    Spawn,
    Exec,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessConfig {
    pub title: String,
    pub cmd: Vec<String>,
    pub link: Option<String>,
    pub cwd: Option<PathBuf>,
    pub watch: Vec<PathBuf>,
    pub watch_settle_time_ms: u64,
    pub mode: PaneMode,
    pub auto_start: bool,
    pub auto_restart: bool,
    pub grace_period: u64,
    pub run_mode: RunMode,
}

impl ProcessConfig {
    pub fn to_std_command(&self) -> Result<std::process::Command, String> {
        if self.cmd.is_empty() {
            return Err(format!("Process command missing for: {}", self.title));
        }
        let mut cmd = std::process::Command::new(&self.cmd[0]);
        if self.cmd.len() > 1 {
            cmd.args(&self.cmd[1..]);
        }
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }
        Ok(cmd)
    }

    pub fn to_pty_command(&self) -> Result<CommandBuilder, String> {
        if self.cmd.is_empty() {
            return Err(format!("Process command missing for: {}", self.title));
        }
        let mut cmd = CommandBuilder::new(&self.cmd[0]);
        if self.cmd.len() > 1 {
            cmd.args(&self.cmd[1..]);
        }
        if let Some(ref cwd) = self.cwd {
            cmd.cwd(cwd);
        } else if let Ok(current_pwd) = std::env::current_dir() {
            cmd.cwd(current_pwd);
        }
        Ok(cmd)
    }
}

impl PinchManifest {
    pub fn resolve_vars(&self, cli_vars: &HashMap<String, String>) -> HashMap<String, String> {
        let mut context_vars = builtin_vars();
        if let Some(user_vars) = &self.vars {
            context_vars.extend(user_vars.clone());
        }
        context_vars.extend(cli_vars.clone());
        context_vars
    }

    pub fn show_vars(
        &self,
        cli_vars: &HashMap<String, String>,
        target: Option<&str>,
        format: Option<OutputFormat>,
    ) -> Result<(), String> {
        let context_vars = self.resolve_vars(cli_vars);

        if let Some(name) = target {
            let Some(val) = context_vars.get(name) else {
                return Err(format!("Variable '{}' not found in configuration", name));
            };
            render_single(val, format)?;
        } else {
            let sorted: BTreeMap<&String, &String> = context_vars.iter().collect();
            render_map(&sorted, format)?;
        }
        Ok(())
    }

    pub fn show_config(&self, cli_vars: &HashMap<String, String>, format: Option<OutputFormat>) -> Result<(), String> {
        let config = self.prepare(cli_vars.clone(), false)?;
        match format.unwrap_or(OutputFormat::Yaml) {
            OutputFormat::Yaml => {
                let yaml_str =
                    serde_yaml::to_string(&config).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
                print!("{}", yaml_str);
            }
            OutputFormat::Json => {
                let json_str =
                    serde_json::to_string_pretty(&config).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
                println!("{}", json_str);
            }
            OutputFormat::Raw | OutputFormat::Properties => {
                println!("{:#?}", config);
            }
        }
        Ok(())
    }

    pub fn show_processes(
        &self,
        cli_vars: &HashMap<String, String>,
        target: Option<&str>,
        format: Option<OutputFormat>,
    ) -> Result<(), String> {
        let config = self.prepare(cli_vars.clone(), false)?;

        if let Some(title) = target {
            let Some(proc) = config.processes.iter().find(|p| p.title == title) else {
                return Err(format!("Process with title '{}' not found in configuration", title));
            };
            let cmd_str = proc.cmd.join(" ");
            render_single(&cmd_str, format)?;
        } else {
            let mut sorted: BTreeMap<String, String> = BTreeMap::new();
            for p in &config.processes {
                sorted.insert(p.title.clone(), p.cmd.join(" "));
            }
            render_map(&sorted, format)?;
        }
        Ok(())
    }

    pub fn list_networks(&self) {
        if let Some(networks) = &self.docker_networks {
            for name in networks.keys() {
                println!("  - {}", name);
            }
        } else {
            println!("  (No networks defined)");
        }
    }

    pub fn show_networks(
        &self,
        cli_vars: &HashMap<String, String>,
        target: Option<&str>,
        format: Option<OutputFormat>,
    ) -> Result<(), String> {
        let context_vars = self.resolve_vars(cli_vars);

        if let Some(networks) = &self.docker_networks {
            if let Some(name) = target {
                let Some(config) = networks.get(name) else {
                    return Err(format!("Network '{}' not found in configuration", name));
                };
                let cmd_args = build_docker_network_command(name, config, &context_vars);
                let cmd_str = format_command_args(cmd_args);
                render_single(&cmd_str, format)?;
            } else {
                let mut sorted: BTreeMap<&String, String> = BTreeMap::new();
                for (name, config) in networks {
                    let cmd_args = build_docker_network_command(name, config, &context_vars);
                    sorted.insert(name, format_command_args(cmd_args));
                }
                render_map(&sorted, format)?;
            }
            Ok(())
        } else if let Some(t) = target {
            Err(format!("Network '{}' not found in configuration", t))
        } else {
            println!("(No Docker networks defined)");
            Ok(())
        }
    }

    pub fn create_networks(&self, cli_vars: &HashMap<String, String>, target: Option<&str>) -> Result<(), String> {
        let context_vars = self.resolve_vars(cli_vars);

        if let Some(networks) = &self.docker_networks {
            for (name, config) in networks {
                if let Some(t) = target {
                    if name != t {
                        continue;
                    }
                }
                ensure_docker_network(name, config, &context_vars)?;
            }
        } else if let Some(t) = target {
            return Err(format!("Network '{}' not found in configuration", t));
        }
        Ok(())
    }

    pub fn list_images(&self, cli_vars: &HashMap<String, String>, format: Option<OutputFormat>) -> Result<(), String> {
        let context_vars = self.resolve_vars(cli_vars);
        let mut images = BTreeSet::new();

        if let Some(processes) = &self.processes {
            for proc in processes {
                if let crate::config::RunManifest::Detailed(crate::config::RunKind::Docker(ref docker_cfg)) = proc.run {
                    let resolved_image = apply_vars(&docker_cfg.image, &context_vars, false);
                    images.insert(resolved_image);
                }
            }
        }

        let chosen_format = format.unwrap_or(OutputFormat::Raw);
        match chosen_format {
            OutputFormat::Raw | OutputFormat::Properties => {
                for img in &images {
                    println!("{}", img);
                }
            }
            OutputFormat::Yaml => {
                let yaml_str =
                    serde_yaml::to_string(&images).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
                print!("{}", yaml_str);
            }
            OutputFormat::Json => {
                let json_str =
                    serde_json::to_string_pretty(&images).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
                println!("{}", json_str);
            }
        }
        Ok(())
    }

    pub fn prepare(&self, cli_vars: HashMap<String, String>, background: bool) -> Result<PinchConfig, String> {
        let context_vars = self.resolve_vars(&cli_vars);

        let title = self.project.name.clone();
        let global_shell = self.shell;
        let global_auto_start = self.auto_start;
        let global_auto_restart = self.auto_restart;
        let global_grace_period = self.grace_period;
        let global_watch_settle = self.watch_settle_time_ms;
        let global_logs_max_size = self.logs_max_size;
        let layout = self.layout.clone().unwrap_or_default();
        let defined_networks = self.docker_networks.clone().unwrap_or_default();
        let default_docker_network = if defined_networks.len() == 1 {
            defined_networks.keys().next().cloned()
        } else {
            None
        };
        let processes_slice = self.processes.as_deref().unwrap_or_default();

        let prepared_processes: Result<Vec<ProcessConfig>, String> = processes_slice
            .iter()
            .map(|raw| {
                let final_cwd = raw
                    .cwd
                    .as_ref()
                    .map(|c| PathBuf::from(apply_vars(c, &context_vars, false)));
                let mut watch_paths = Vec::new();
                if let Some(watches) = &raw.watch {
                    for w in watches {
                        watch_paths.push(PathBuf::from(apply_vars(w, &context_vars, false)));
                    }
                }
                let run_ctx = RunContext {
                    title: &raw.title,
                    vars: &context_vars,
                    global_shell,
                    default_docker_network: default_docker_network.as_ref(),
                    defined_networks: &defined_networks,
                    background,
                };
                let built = raw.run.build_command(&run_ctx)?;

                let link = raw.link.as_ref().map(|l| apply_vars(l, &context_vars, false));
                let watch_settle_time_ms = raw.watch_settle_time_ms.or(global_watch_settle).unwrap_or(800);
                Ok(ProcessConfig {
                    title: raw.title.clone(),
                    cmd: built.cmd,
                    link,
                    cwd: final_cwd,
                    watch: watch_paths,
                    watch_settle_time_ms,
                    mode: raw.mode.unwrap_or(PaneMode::Log),
                    auto_start: raw.auto_start.or(global_auto_start).unwrap_or(true),
                    auto_restart: raw.auto_restart.or(global_auto_restart).unwrap_or(true),
                    grace_period: raw.grace_period.or(global_grace_period).unwrap_or(3000),
                    run_mode: built.run_mode,
                })
            })
            .collect();

        Ok(PinchConfig {
            title,
            processes: prepared_processes?,
            logs_max_size: global_logs_max_size,
            layout,
        })
    }
}

fn render_single<T: Serialize + std::fmt::Display>(value: &T, format: Option<OutputFormat>) -> Result<(), String> {
    match format.unwrap_or(OutputFormat::Raw) {
        OutputFormat::Raw | OutputFormat::Properties => println!("{}", value),
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(value).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(value).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}

fn render_map<K, V>(map: &BTreeMap<K, V>, format: Option<OutputFormat>) -> Result<(), String>
where
    K: std::fmt::Display + Serialize,
    V: std::fmt::Display + Serialize,
{
    match format.unwrap_or(OutputFormat::Yaml) {
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(map).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Raw => {
            for (key, val) in map {
                println!("{}\t{}", key, val);
            }
        }
        OutputFormat::Properties => {
            for (key, val) in map {
                println!("{}={}", key, val);
            }
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(map).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}

fn format_command_args(args: Vec<String>) -> String {
    args.into_iter()
        .map(|arg| {
            shlex::try_quote(&arg)
                .unwrap_or(std::borrow::Cow::Borrowed(&arg))
                .to_string()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn build_docker_network_command(
    net_key: &str,
    config: &DockerNetworkConfig,
    vars: &HashMap<String, String>,
) -> Vec<String> {
    let net_name = apply_vars(net_key, vars, false);
    let mut cmd_args = vec!["docker".to_string(), "network".to_string(), "create".to_string()];
    cmd_args.push("-o".to_string());
    cmd_args.push(format!("com.docker.network.bridge.name={}", net_name));
    cmd_args.push("--subnet".to_string());
    cmd_args.push(apply_vars(config.subnet(), vars, false));
    cmd_args.push("-d".to_string());
    cmd_args.push("bridge".to_string());
    if let Some(args_list) = config.args() {
        for arg in args_list {
            cmd_args.push(apply_vars(arg, vars, false));
        }
    }
    cmd_args.push(net_name);
    cmd_args
}

fn ensure_docker_network(
    net_key: &str,
    config: &DockerNetworkConfig,
    vars: &HashMap<String, String>,
) -> Result<(), String> {
    use std::process::Command;
    let net_name = apply_vars(net_key, vars, false);
    let status = Command::new("docker")
        .args(["network", "inspect", &net_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("Failed to inspect docker network '{}': {}", net_name, e))?;
    if !status.success() {
        let mut cmd = Command::new("docker");
        let cmd_args = build_docker_network_command(net_key, config, vars);
        cmd.args(&cmd_args[1..]);
        let create_status = cmd
            .status()
            .map_err(|e| format!("Failed to create docker network '{}': {}", net_name, e))?;
        if !create_status.success() {
            return Err(format!("Docker failed to create network: {}", net_name));
        }
    }
    Ok(())
}
