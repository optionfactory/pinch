mod docker;
mod docker_intrude;
mod process;

use crate::config::{DockerNetworkConfig, ProcessRunConfig, RunKind, RunManifest, RunMode};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutput {
    pub cmd: Vec<String>,
    pub run_mode: RunMode,
}

pub type BuildResult = Result<BuildOutput, String>;

pub struct RunContext<'a> {
    pub title: &'a str,
    pub vars: &'a HashMap<String, String>,
    pub global_shell: Option<bool>,
    pub default_docker_network: Option<&'a String>,
    pub defined_networks: &'a HashMap<String, DockerNetworkConfig>,
    pub background: bool,
}

pub trait RunBuilder {
    fn build_command(&self, ctx: &RunContext) -> BuildResult;
}

impl RunBuilder for RunManifest {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        match self {
            RunManifest::Shorthand(cmd) => {
                let process_run = ProcessRunConfig {
                    bash: ctx.global_shell.unwrap_or(false),
                    cmd: cmd.clone(),
                };
                process_run.build_command(ctx)
            }
            RunManifest::Detailed(kind) => kind.build_command(ctx),
        }
    }
}

impl RunBuilder for RunKind {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        match self {
            RunKind::Process(run_cfg) => run_cfg.build_command(ctx),
            RunKind::Docker(run_cfg) => run_cfg.build_command(ctx),
            RunKind::DockerIntrude(run_cfg) => run_cfg.build_command(ctx),
        }
    }
}

pub fn parse_command_string(cmd: &str, bash: bool, title: &str) -> Result<Vec<String>, String> {
    if bash {
        Ok(vec!["bash".to_string(), "-c".to_string(), cmd.to_string()])
    } else {
        shlex::split(cmd).ok_or_else(|| format!("Failed to parse command for '{}': {}", title, cmd))
    }
}
