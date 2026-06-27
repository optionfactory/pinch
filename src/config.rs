use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaneMode {
    Log,
    Tui,
}

#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub network: String,
    pub ip: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutSplit {
    pub title: String,
    pub size_percentage: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutBlock {
    pub title: Option<String>,
    pub edge: LayoutEdge,
    pub size_percentage: u16,
    pub direction: Option<String>,
    pub splits: Option<Vec<LayoutSplit>>,
}

#[derive(Debug, Deserialize)]
pub struct RawPinchConfig {
    pub title: Option<String>,
    pub vars: Option<HashMap<String, String>>,
    pub processes: Vec<RawProcessConfig>,
    pub logs_max_size: Option<usize>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub grace_period: Option<u64>,
    pub shell: Option<bool>,
    pub docker_network: Option<String>,
    pub watch_settle_time_ms: Option<u64>,
    pub layout: Option<Vec<LayoutBlock>>,
}

#[derive(Debug, Deserialize)]
pub struct RawProcessConfig {
    pub title: String,
    pub cmd: String,
    pub cwd: Option<String>,
    pub watch: Option<Vec<String>>,
    pub watch_settle_time_ms: Option<u64>,
    pub mode: Option<PaneMode>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub grace_period: Option<u64>,
    pub shell: Option<bool>,
    pub docker_network: Option<String>,
    pub docker_ip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PinchConfig {
    pub title: String,
    pub processes: Vec<ProcessConfig>,
    pub logs_max_size: Option<usize>,
    pub layout: Vec<LayoutBlock>,
}

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub title: String,
    pub cmd: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub watch: Vec<PathBuf>,
    pub watch_settle_time_ms: u64,
    pub mode: PaneMode,
    pub auto_start: bool,
    pub auto_restart: bool,
    pub grace_period: u64,
}

impl RawPinchConfig {
    pub fn prepare(self) -> PinchConfig {
        let mut context_vars = builtin_vars();

        if let Some(user_vars) = self.vars {
            context_vars.extend(user_vars);
        }

        let title = self.title.unwrap_or_else(|| "Process Supervisor".to_string());
        let global_shell = self.shell;
        let global_docker_network = self.docker_network;
        let global_auto_start = self.auto_start;
        let global_auto_restart = self.auto_restart;
        let global_grace_period = self.grace_period;
        let global_watch_settle = self.watch_settle_time_ms;
        let global_logs_max_size = self.logs_max_size;
        let layout = self.layout.unwrap_or_default();

        let prepared_processes = self
            .processes
            .into_iter()
            .map(|raw| {
                let final_cwd = raw.cwd.map(|c| PathBuf::from(apply_vars(&c, &context_vars, false)));

                let mut watch_paths = Vec::new();
                if let Some(watches) = raw.watch {
                    for w in watches {
                        watch_paths.push(PathBuf::from(apply_vars(&w, &context_vars, false)));
                    }
                }

                let resolved_docker = match raw.docker_ip {
                    Some(ip) => {
                        let network = raw
                            .docker_network
                            .clone()
                            .or_else(|| global_docker_network.clone())
                            .unwrap_or_else(|| {
                                panic!(
                                    "Process '{}' has a docker_ip, but no docker_network was found locally or globally!",
                                    raw.title
                                )
                            });
                        Some(DockerConfig { network, ip })
                    }
                    None => None,
                };

                let use_shell = raw.shell.or(global_shell).unwrap_or(false);

                let expanded_cmd = apply_vars(raw.cmd.trim(), &context_vars, true);

                let final_cmd = if let Some(docker) = &resolved_docker {
                    let sanitized_title: String = raw
                        .title
                        .replace('_', "-")
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                        .collect();
                    let container_name = format!("pinch-ns-{}", sanitized_title.to_lowercase());

                    let mut cmd_vec = vec![
                        "docker-intrude".to_string(),
                        "--name".to_string(),
                        container_name,
                        "--net".to_string(),
                        docker.network.clone(),
                        "--ip".to_string(),
                        docker.ip.clone(),
                        "--".to_string(),
                    ];

                    if use_shell {
                        cmd_vec.push("bash".to_string());
                        cmd_vec.push("-c".to_string());
                        cmd_vec.push(expanded_cmd);
                    } else {
                        let tokens = shlex::split(&expanded_cmd).unwrap_or_else(|| panic!("Failed to parse command: {}", expanded_cmd));
                        cmd_vec.extend(tokens);
                    }

                    cmd_vec
                } else if use_shell {
                    vec!["bash".to_string(), "-c".to_string(), expanded_cmd]
                } else {
                    shlex::split(&expanded_cmd).unwrap_or_else(|| panic!("Failed to parse command: {}", expanded_cmd))
                };

                let watch_settle_time_ms = raw.watch_settle_time_ms.or(global_watch_settle).unwrap_or(800);

                ProcessConfig {
                    title: raw.title,
                    cmd: final_cmd,
                    cwd: final_cwd,
                    watch: watch_paths,
                    watch_settle_time_ms,
                    mode: raw.mode.unwrap_or(PaneMode::Log),
                    auto_start: raw.auto_start.or(global_auto_start).unwrap_or(true),
                    auto_restart: raw.auto_restart.or(global_auto_restart).unwrap_or(true),
                    grace_period: raw.grace_period.or(global_grace_period).unwrap_or(3000),
                }
            })
            .collect();

        PinchConfig {
            title,
            processes: prepared_processes,
            logs_max_size: global_logs_max_size,
            layout,
        }
    }
}

fn builtin_vars() -> HashMap<String, String> {
    let mut builtins = HashMap::new();

    if let Ok(current_pwd) = std::env::current_dir() {
        builtins.insert("pwd".to_string(), current_pwd.to_string_lossy().into_owned());
    }
    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        builtins.insert("user".to_string(), user);
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        builtins.insert("home".to_string(), home);
    }

    builtins
}

fn apply_vars(text: &str, vars: &HashMap<String, String>, quote_spaces: bool) -> String {
    let mut result = String::new();
    let mut rest = text;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];

        if let Some(end) = after_open.find("}}") {
            let key = after_open[..end].trim();

            if let Some(val) = vars.get(key) {
                if quote_spaces && val.contains(' ') {
                    result.push('"');
                    result.push_str(val);
                    result.push('"');
                } else {
                    result.push_str(val);
                }
            } else {
                result.push_str("{{");
                result.push_str(&after_open[..end + 2]);
            }
            rest = &after_open[end + 2..];
        } else {
            result.push_str("{{");
            rest = after_open;
            break;
        }
    }
    result.push_str(rest);
    result
}
