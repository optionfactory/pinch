use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[doc = "Root configuration manifest for a Pinch project (`pinch.yaml`)."]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinchManifest {
    #[doc = "Explicit schema version for manifest compatibility (must be 1)."]
    #[schemars(schema_with = "schema_version_schema")]
    pub schema_version: u32,

    #[doc = "Name of the project."]
    #[schemars(schema_with = "identifier_schema")]
    pub name: String,

    #[doc = "Custom configuration variables available for string expansion (e.g., `{{var_name}}`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HashMap<String, String>")]
    pub vars: Option<HashMap<String, String>>,

    #[doc = "List of supervised processes, containers, and background services to execute."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<ProcessManifest>")]
    pub processes: Option<Vec<ProcessManifest>>,

    #[doc = "Maximum number of log lines to retain in memory per process pane."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub logs_max_size: Option<usize>,

    #[doc = "Whether processes start automatically upon launching the supervisor (default: `true`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_start: Option<bool>,

    #[doc = "Whether processes restart automatically if they exit or crash (default: `true`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_restart: Option<bool>,

    #[doc = "Delay in milliseconds before automatically restarting a process after an exit (default: `3000`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub grace_period: Option<u64>,

    #[doc = "If present, executes commands using the configed shell '-c' option globally."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "WrappingShell")]
    pub shell: Option<WrappingShell>,

    #[doc = "Custom Docker bridge networks managed and initialized by Pinch."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HashMap<String, DockerNetworkConfig>")]
    pub docker_networks: Option<HashMap<String, DockerNetworkConfig>>,

    #[doc = "Debounce delay in milliseconds when watching files for auto-restarting (default: `800`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub watch_settle_time_ms: Option<u64>,

    #[doc = "Progressive edge-carving layout rules defining how process panes are arranged."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<LayoutNode>")]
    pub layout: Option<Vec<LayoutNode>>,
}

#[doc = "Shell to be used to run commands."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum WrappingShell {
    Bash,
    Zsh,
    Fish,
}

#[doc = "Configuration for an individual supervised process or container."]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessManifest {
    #[doc = "Name of the process."]
    #[schemars(schema_with = "identifier_schema")]
    pub name: String,

    #[doc = "Display name of the process pane in the TUI dashboard (defaults to name)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,

    #[doc = "Execution definition specifying how the command or container is spawned."]
    pub run: RunManifest,

    #[doc = "Working directory for process execution (supports variable expansion like `{{pwd}}`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub cwd: Option<String>,

    #[doc = "Optional web URL or link associated with this process (clickable in the TUI header)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub link: Option<String>,

    #[doc = "List of file or directory paths that trigger an automatic restart when modified."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub watch: Option<Vec<String>>,

    #[doc = "Process-specific debounce delay in milliseconds for file watching."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub watch_settle_time_ms: Option<u64>,

    #[doc = "Display mode for the process output in the TUI dashboard (`log` or `tui`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "PaneMode")]
    pub mode: Option<PaneMode>,

    #[doc = "Override global auto-start default for this specific process."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_start: Option<bool>,

    #[doc = "Override global auto-restart default for this specific process."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_restart: Option<bool>,

    #[doc = "Override global grace period delay in milliseconds for this specific process."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub grace_period: Option<u64>,
}

#[doc = "How the process command is specified in YAML."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum RunManifest {
    #[doc = "Simple shell command string executed as a local OS process."]
    Shorthand(String),
    #[doc = "Explicit execution configuration specifying process type (`process`, `docker`, or `docker-intrude`)."]
    Detailed(RunKind),
}

#[doc = "Explicit runtime environment type for a supervised process."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum RunKind {
    #[doc = "Spawns a standard local OS process."]
    Process(ProcessRunConfig),
    #[doc = "Spawns a managed container using `docker run --rm` (or `podman run --rm` via `engine`)."]
    Docker(DockerRunConfig),
    #[doc = "Spawns a local host binary attached inside a Docker bridge network namespace (`docker-intrude`)."]
    DockerIntrude(DockerIntrudeRunConfig),
}

#[doc = "Configuration for executing a local OS process."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessRunConfig {
    #[doc = "If present, executes commands using the configed shell '-c' option."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "WrappingShell")]
    pub shell: Option<WrappingShell>,
    #[doc = "The command string to execute."]
    pub cmd: String,
    #[doc = "Directories to remap through `docker-bluff` (Linux idmapped mounts), one per `--map` (`SRC[:DST]`, supports variable expansion), so a container-style UID and the host each see the mounted files as their own. Requires `remap_ids`."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub remap_paths: Option<Vec<String>>,
    #[doc = "Id swaps `docker-bluff` applies, one per `--id` (e.g. `me:0`, `u:0:33`, `g:0:33`; `me` resolves to the invoking user's uid/gid). Setting it runs the command under docker-bluff so mounted files are owned correctly on both sides."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub remap_ids: Option<Vec<String>>,
}

#[doc = "Container engine binary used to spawn a container."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum ContainerEngine {
    Docker,
    Podman,
}

#[doc = "Configuration for executing a container (same shape as the `container` block of `optionfactory.services.bundle`)."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerRunConfig {
    #[doc = "Container engine used to run the image (`docker` or `podman`, default `docker`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ContainerEngine")]
    pub engine: Option<ContainerEngine>,
    #[doc = "Container name passed as `--name` (defaults to the enclosing process name)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "identifier_schema")]
    pub name: Option<String>,
    #[doc = "The container image reference to run."]
    pub image: String,
    #[doc = "Raw flags appended verbatim to the engine run command, after the rendered ones."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub opts: Option<String>,
    #[doc = "Arguments passed to the container's entrypoint."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub args: Option<String>,
    #[doc = "Network to attach the container to (rendered as `--network`, empty values ignored)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub network: Option<String>,
    #[doc = "Static IP address assigned to the container on the bridge network (rendered as `--ip`, empty values ignored)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub ip: Option<String>,
    #[doc = "Environment variables passed to the container (a `KEY: value` mapping, rendered as `--env`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HashMap<String, String>")]
    pub env: Option<HashMap<String, String>>,
    #[doc = "Ports published to the host (rendered as `-p`, empty entries ignored)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub publish: Option<Vec<String>>,
    #[doc = "Mounts attached to the container (rendered as `--mount type=<type>,source=<source>,target=<target>[,readonly][,bind-create-src][,<opts>]`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<DockerMountConfig>")]
    pub mounts: Option<Vec<DockerMountConfig>>,
    #[doc = "Volumes attached to the container (rendered as `--volume`, empty entries ignored)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub volumes: Option<Vec<String>>,
    #[doc = "Id swaps `docker-bluff` applies to the container's bind mounts, one per `--id` (e.g. `me:0`, `u:0:33`, `g:0:33`; `me` resolves to the invoking user's uid/gid). When set, `docker run` is launched under docker-bluff so its `-v`/`--mount` sources are idmapped. (There is no `remap_paths` for `type: docker`, docker-bluff discovers the run's mounts itself.)"]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub remap_ids: Option<Vec<String>>,
}

#[doc = "Configuration for a single container mount."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerMountConfig {
    #[doc = "Mount type (default `bind`)."]
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub mount_type: Option<String>,
    #[doc = "Source path of the mount."]
    pub source: String,
    #[doc = "Target path inside the container (defaults to `source`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub target: Option<String>,
    #[doc = "Whether the mount is readonly (default `true`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub readonly: Option<bool>,
    #[doc = "Create the bind source directory (and missing parents) on the host when it does not exist (default `false`). Rendered as the `bind-create-src` mount option: docker (29+) creates it owned by root; with `remap_ids`, `docker-bluff` creates it owned by the invoking user instead. Only for `type: bind`."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub create: Option<bool>,
    #[doc = "Comma-separated extra mount options appended verbatim (e.g., `bind-propagation=rshared`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub opts: Option<String>,
}

#[doc = "Configuration for executing a host binary inside a Docker network namespace."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerIntrudeRunConfig {
    #[doc = "Static IP address assigned to the namespace container on the target bridge subnet."]
    pub ip: String,
    #[doc = "Target Docker network name (optional if exactly one network is defined in `docker_networks`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub network: Option<String>,
    #[doc = "If present, executes commands using the configed shell '-c' option."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "WrappingShell")]
    pub shell: Option<WrappingShell>,
    #[doc = "Host command string to execute within the target network namespace."]
    pub cmd: String,
    #[doc = "Directories to remap through `docker-bluff` (Linux idmapped mounts), one per `--map` (`SRC[:DST]`, supports variable expansion), so a container-style UID and the host each see the mounted files as their own. Requires `remap_ids`."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub remap_paths: Option<Vec<String>>,
    #[doc = "Id swaps `docker-bluff` applies, one per `--id` (e.g. `me:0`, `u:0:33`, `g:0:33`; `me` resolves to the invoking user's uid/gid). Setting it runs the command under docker-bluff so mounted files are owned correctly on both sides."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub remap_ids: Option<Vec<String>>,
}

#[doc = "Configuration for creating a Docker bridge network."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum DockerNetworkConfig {
    #[doc = "CIDR subnet string for the bridge network (e.g., `\"172.18.23.0/24\"`)."]
    Simple(String),
    #[doc = "Explicit network configuration specifying subnet CIDR and custom Docker creation arguments."]
    Detailed {
        subnet: String,
        #[doc = "Raw flags appended verbatim to `docker network create` (shell-style string, same as a process `opts`)."]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        args: Option<String>,
    },
}

impl DockerNetworkConfig {
    pub fn subnet(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Detailed { subnet, .. } => subnet,
        }
    }

    pub fn args(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { args, .. } => args.as_deref(),
        }
    }
}

#[doc = "Display rendering mode for a process pane."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum PaneMode {
    Log,
    Tui,
}

#[doc = "A recursive edge-carving and splitting node for arranging terminal panes."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("oneOf" = [
    {
        "description": "Leaf node targeting a specific process name or 'combined-logs'",
        "required": ["name"],
        "not": { "anyOf": [{ "required": ["unassigned"] }, { "required": ["items"] }] }
    },
    {
        "description": "Leaf node acting as the container for unassigned process panes",
        "required": ["unassigned"],
        "not": { "anyOf": [{ "required": ["name"] }, { "required": ["items"] }] }
    },
    {
        "description": "Branch node splitting screen area recursively among child nodes",
        "required": ["items"],
        "not": { "anyOf": [{ "required": ["name"] }, { "required": ["unassigned"] }] }
    }
]))]
pub struct LayoutNode {
    #[doc = "Side of the remaining terminal space to carve from (`top`, `bottom`, `left`, `right`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "LayoutEdge")]
    pub edge: Option<LayoutEdge>,

    #[doc = "Percentage of available space to allocate (0 to 100). Auto-calculated among siblings if omitted."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u16")]
    #[schemars(range(min = 0, max = 100))]
    pub size: Option<u16>,

    #[doc = "Split orientation for child panes inside this node (`horizontal` or `vertical`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "LayoutDirection")]
    pub direction: Option<LayoutDirection>,

    #[doc = "Target process name to place inside this pane (or `\"combined-logs\"`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "identifier_schema")]
    pub name: Option<String>,

    #[doc = "If true, automatically places all unassigned process panes inside this node."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub unassigned: Option<bool>,

    #[doc = "Layout nodes to recursively arrange within this node."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<LayoutNode>>,
}

#[doc = "Target edge of the terminal space to carve from."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum LayoutEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[doc = "Split orientation for sub-panes (`horizontal` or `vertical`)."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

fn schema_version_schema(_generator: &mut SchemaGenerator) -> Schema {
    let schema_val = serde_json::json!({
        "type": "integer",
        "const": 1,
        "description": "Explicit schema version for manifest compatibility (must be 1)."
    });
    serde_json::from_value(schema_val).expect("valid schema")
}

fn identifier_schema(_generator: &mut SchemaGenerator) -> Schema {
    let schema_val = serde_json::json!({
        "type": "string",
        "pattern": "^[A-Za-z0-9-_]+$",
        "description": "A valid identifier."
    });
    serde_json::from_value(schema_val).expect("valid schema")
}
