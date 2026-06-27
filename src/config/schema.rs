use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinchManifest {
    pub project: ProjectManifest,
    pub vars: Option<HashMap<String, String>>,
    pub processes: Option<Vec<ProcessManifest>>,
    pub logs_max_size: Option<usize>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub grace_period: Option<u64>,
    pub shell: Option<bool>,
    pub docker_networks: Option<HashMap<String, DockerNetworkConfig>>,
    pub watch_settle_time_ms: Option<u64>,
    pub layout: Option<Vec<LayoutBlock>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessManifest {
    pub title: String,
    pub run: RunManifest,
    pub cwd: Option<String>,
    pub link: Option<String>,
    pub watch: Option<Vec<String>>,
    pub watch_settle_time_ms: Option<u64>,
    pub mode: Option<PaneMode>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub grace_period: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum RunManifest {
    Shorthand(String),
    Detailed(RunKind),
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum RunKind {
    Process(ProcessRunConfig),
    Docker(DockerRunConfig),
    DockerIntrude(DockerIntrudeRunConfig),
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessRunConfig {
    #[serde(default)]
    pub bash: bool,
    pub cmd: String,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerRunConfig {
    pub image: String,
    pub opts: Option<String>,
    pub args: Option<String>,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerIntrudeRunConfig {
    pub ip: String,
    pub network: Option<String>,
    #[serde(default)]
    pub bash: bool,
    pub cmd: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutBlock {
    pub title: Option<String>,
    pub edge: LayoutEdge,
    pub size_percentage: u16,
    pub direction: Option<String>,
    pub splits: Option<Vec<LayoutSplit>>,
    pub unassigned: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum PaneMode {
    Log,
    Tui,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutSplit {
    pub title: Option<String>,
    pub size_percentage: u16,
    pub unassigned: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum LayoutEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum DockerNetworkConfig {
    Simple(String),
    Detailed { subnet: String, args: Option<Vec<String>> },
}

impl DockerNetworkConfig {
    pub fn subnet(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Detailed { subnet, .. } => subnet,
        }
    }
    pub fn args(&self) -> Option<&[String]> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { args, .. } => args.as_deref(),
        }
    }
}
