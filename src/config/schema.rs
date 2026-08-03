use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[doc = "Root configuration manifest for a Pinch project (`pinch.yaml`)."]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinchManifest {
    #[doc = "Core project metadata, governance attributes, and architecture classification."]
    pub project: ProjectManifest,
    #[doc = "Custom configuration variables available for string expansion (e.g., `{{var_name}}`)."]
    pub vars: Option<HashMap<String, String>>,
    #[doc = "List of supervised processes, containers, and background services to execute."]
    pub processes: Option<Vec<ProcessManifest>>,
    #[doc = "Maximum number of log lines to retain in memory per process pane."]
    pub logs_max_size: Option<usize>,
    #[doc = "Whether processes start automatically upon launching the supervisor (default: `true`)."]
    pub auto_start: Option<bool>,
    #[doc = "Whether processes restart automatically if they exit or crash (default: `true`)."]
    pub auto_restart: Option<bool>,
    #[doc = "Delay in milliseconds before automatically restarting a process after an exit (default: `3000`)."]
    pub grace_period: Option<u64>,
    #[doc = "If true, executes shorthand command strings using `bash -c` globally (default: `false`)."]
    pub shell: Option<bool>,
    #[doc = "Custom Docker bridge networks managed and initialized by Pinch."]
    pub docker_networks: Option<HashMap<String, DockerNetworkConfig>>,
    #[doc = "Debounce delay in milliseconds when watching files for auto-restarting (default: `800`)."]
    pub watch_settle_time_ms: Option<u64>,
    #[doc = "Progressive edge-carving layout rules defining how process panes are arranged."]
    pub layout: Option<Vec<LayoutBlock>>,
}

#[doc = "High-level project identification, governance, and architecture classification."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    #[doc = "Human-readable project title displayed at the top of the TUI dashboard."]
    pub name: String,

    #[doc = "Primary architectural role of the project."]
    #[serde(rename = "type")]
    pub project_type: Option<ProjectType>,
    #[doc = "Network ingress reachability for the project."]
    pub exposure: Option<ExposureType>,
    #[doc = "Operational maintenance status of the repository."]
    pub lifecycle: Option<LifecycleType>,
    #[doc = "Primary authentication mechanism for ingress requests."]
    pub auth: Option<AuthType>,
    #[doc = "Sensitive data classifications handled by the service."]
    pub sensitivity: Option<Vec<Sensitivity>>,
    #[doc = "List of deployment environments supported by this project (e.g., `dev`, `staging`, `prod`)."]
    pub environments: Option<Vec<String>>,
}

#[doc = "Primary architectural role of the project."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum ProjectType {
    #[doc = "Shared code dependency imported by other projects."]
    Library,
    #[doc = "Long-running server, API, or backend daemon."]
    Service,
    #[doc = "CLI utility or internal developer tool."]
    Tool,
    #[doc = "Short-lived batch process, cron, or CI/CD script."]
    Job,
}

#[doc = "Network ingress reachability for the project."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum ExposureType {
    #[doc = "Reachable from the public internet or Cloudflare edge."]
    Internet,
    #[doc = "Reachable only inside the company VPN, VPC, or internal service mesh."]
    Internal,
    #[doc = "No incoming network traffic (e.g., background worker or CLI tool)."]
    None,
}

#[doc = "Operational maintenance status of the repository."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum LifecycleType {
    #[doc = "Actively developed and supported in production."]
    Active,
    #[doc = "Scheduled for decommissioning; do not add new dependencies."]
    Deprecated,
    #[doc = "In production but receiving only critical bug/security fixes."]
    Maintenance,
    #[doc = "Experimental proof-of-concept; no production SLA."]
    Prototype,
}

#[doc = "Sensitive data classifications handled by the service."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum Sensitivity {
    #[doc = "Non-sensitive data safe for public distribution."]
    Public,
    #[doc = "Standard internal business data."]
    Internal,
    #[doc = "Restricted company intellectual property or trade secrets."]
    Confidential,
    #[doc = "Personally Identifiable Information (names, emails, addresses)."]
    Pii,
    #[doc = "Payment Card Industry data (credit cards, billing details)."]
    Pci,
    #[doc = "Protected Health Information (medical records, insurance claims)."]
    Phi,
}

#[doc = "Primary authentication mechanism for ingress requests."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum AuthType {
    #[doc = "Identity provider."]
    Idp,
    #[doc = "Generic OAuth 2.0."]
    Oauth2,
    #[doc = "Generic SAML."]
    Saml,
    #[doc = "Mutual TLS client certificate authentication."]
    Mtls,
    #[doc = "Custom token authentication."]
    Token,
    #[doc = "HTTP Basic authentication (legacy/internal)."]
    Basic,
    #[doc = "Custom form authentication."]
    Form,
    #[doc = "Unprotected public endpoint (no authentication required)."]
    None,
}

#[doc = "Configuration for an individual supervised process or container."]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessManifest {
    #[doc = "Display name of the process pane in the TUI dashboard."]
    pub title: String,
    #[doc = "Execution definition specifying how the command or container is spawned."]
    pub run: RunManifest,
    #[doc = "Working directory for process execution (supports variable expansion like `{{pwd}}`)."]
    pub cwd: Option<String>,
    #[doc = "Optional web URL or link associated with this process (clickable in the TUI header)."]
    pub link: Option<String>,
    #[doc = "List of file or directory paths that trigger an automatic restart when modified."]
    pub watch: Option<Vec<String>>,
    #[doc = "Process-specific debounce delay in milliseconds for file watching."]
    pub watch_settle_time_ms: Option<u64>,
    #[doc = "Display mode for the process output in the TUI dashboard (`log` or `tui`)."]
    pub mode: Option<PaneMode>,
    #[doc = "Override global auto-start default for this specific process."]
    pub auto_start: Option<bool>,
    #[doc = "Override global auto-restart default for this specific process."]
    pub auto_restart: Option<bool>,
    #[doc = "Override global grace period delay in milliseconds for this specific process."]
    pub grace_period: Option<u64>,
}

#[doc = "How the process command is specified in YAML."]
#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum RunManifest {
    #[doc = "Simple shell command string executed as a local OS process."]
    Shorthand(String),
    #[doc = "Explicit execution configuration specifying process type (`process`, `docker`, or `docker-intrude`)."]
    Detailed(RunKind),
}

#[doc = "Explicit runtime environment type for a supervised process."]
#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum RunKind {
    #[doc = "Spawns a standard local OS process."]
    Process(ProcessRunConfig),
    #[doc = "Spawns a managed Docker container using `docker run --rm`."]
    Docker(DockerRunConfig),
    #[doc = "Spawns a local host binary attached inside a Docker bridge network namespace (`docker-intrude`)."]
    DockerIntrude(DockerIntrudeRunConfig),
}

#[doc = "Configuration for executing a local OS process."]
#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessRunConfig {
    #[doc = "Whether to wrap the command string in `bash -c` (default: `false`)."]
    #[serde(default)]
    pub bash: bool,
    #[doc = "The command string to execute."]
    pub cmd: String,
}

#[doc = "Configuration for executing a Docker container."]
#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerRunConfig {
    #[doc = "The Docker image reference to run."]
    pub image: String,
    #[doc = "Arguments passed to `docker run --rm` (e.g., `--name`, `--network`, `--ip`)."]
    pub opts: Option<String>,
    #[doc = "Arguments passed to the container's entrypoint."]
    pub args: Option<String>,
}

#[doc = "Configuration for executing a host binary inside a Docker network namespace."]
#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerIntrudeRunConfig {
    #[doc = "Static IP address assigned to the namespace container on the target bridge subnet."]
    pub ip: String,
    #[doc = "Target Docker network name (optional if exactly one network is defined in `docker_networks`)."]
    pub network: Option<String>,
    #[doc = "Whether to wrap the command string in `bash -c` (default: `false`)."]
    #[serde(default)]
    pub bash: bool,
    #[doc = "Host command string to execute within the target network namespace."]
    pub cmd: String,
}

#[doc = "A progressive edge-carving block ruleset for arranging terminal panes."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutBlock {
    #[doc = "Target process title to place inside this block (or `\"Combined Logs\"`)."]
    pub title: Option<String>,
    #[doc = "Side of the remaining terminal space to carve from (`top`, `bottom`, `left`, `right`)."]
    pub edge: LayoutEdge,
    #[doc = "Percentage of currently available space to allocate (0 to 100)."]
    pub size_percentage: u16,
    #[doc = "Split orientation for sub-panes (`horizontal` or `vertical`)."]
    pub direction: Option<String>,
    #[doc = "Sub-panes to arrange within this carved edge block."]
    pub splits: Option<Vec<LayoutSplit>>,
    #[doc = "If true, automatically places all unassigned process panes inside this block."]
    pub unassigned: Option<bool>,
}

#[doc = "Display rendering mode for a process pane."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum PaneMode {
    #[doc = "Standard streaming log tailer with wrap and truncation controls."]
    Log,
    #[doc = "Allocates a PTY for interactive terminal applications (`top`, `vim`, `htop`)."]
    Tui,
}

#[doc = "A sub-pane division within an edge-carved layout block."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutSplit {
    #[doc = "Target process title to place inside this split (or `\"Combined Logs\"`)."]
    pub title: Option<String>,
    #[doc = "Percentage of space within the parent block to allocate (0 to 100)."]
    pub size_percentage: u16,
    #[doc = "If true, automatically places all unassigned process panes inside this split."]
    pub unassigned: Option<bool>,
}

#[doc = "Target edge of the terminal space to carve from."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum LayoutEdge {
    #[doc = "Carve from the top edge."]
    Top,
    #[doc = "Carve from the bottom edge."]
    Bottom,
    #[doc = "Carve from the left edge."]
    Left,
    #[doc = "Carve from the right edge."]
    Right,
}

#[doc = "Configuration for creating a Docker bridge network."]
#[derive(Debug, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum DockerNetworkConfig {
    #[doc = "CIDR subnet string for the bridge network (e.g., `\"172.18.23.0/24\"`)."]
    Simple(String),
    #[doc = "Explicit network configuration specifying subnet CIDR and custom Docker creation arguments."]
    Detailed {
        #[doc = "CIDR subnet string for the bridge network."]
        subnet: String,
        #[doc = "Custom CLI arguments passed to `docker network create`."]
        args: Option<Vec<String>>,
    },
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
