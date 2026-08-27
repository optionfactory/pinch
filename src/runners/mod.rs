mod docker;
mod docker_intrude;
mod process;
mod remap;

use crate::{
    config::{ContainerRef, DockerNetworkConfig, ProcessRunConfig, RunKind, RunManifest, RunMode},
    schema::WrappingShell,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutput {
    pub cmd: Vec<String>,
    pub run_mode: RunMode,
    /// Set for docker-type processes only.
    pub container: Option<ContainerRef>,
}

pub type BuildResult = Result<BuildOutput, String>;

pub use remap::wrap_with_docker_bluff;

pub struct RunContext<'a> {
    pub name: &'a str,
    pub vars: &'a HashMap<String, String>,
    pub global_shell: Option<WrappingShell>,
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
                    shell: ctx.global_shell,
                    cmd: cmd.clone(),
                    remap_paths: None,
                    remap_ids: None,
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

pub fn parse_command_string(cmd: &str, shell: Option<WrappingShell>, name: &str) -> Result<Vec<String>, String> {
    match shell {
        Some(WrappingShell::Bash) => Ok(vec!["bash".to_string(), "-c".to_string(), cmd.to_string()]),
        Some(WrappingShell::Zsh) => Ok(vec!["zsh".to_string(), "-c".to_string(), cmd.to_string()]),
        Some(WrappingShell::Fish) => Ok(vec!["fish".to_string(), "-c".to_string(), cmd.to_string()]),
        None => shlex::split(cmd).ok_or_else(|| format!("Failed to parse command for '{}': {}", name, cmd)),
    }
}
