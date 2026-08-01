use crate::config::{LayoutBlock, PaneMode, PinchManifest};
use crate::runners::{RunBuilder, RunContext};
use crate::vars::apply_vars;
use crate::vars::builtin_vars;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

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

impl PinchManifest {
    pub fn resolve_vars(&self, cli_vars: &HashMap<String, String>) -> HashMap<String, String> {
        let mut context_vars = builtin_vars();
        if let Some(user_vars) = &self.vars {
            context_vars.extend(user_vars.clone());
        }
        context_vars.extend(cli_vars.clone());
        context_vars
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
