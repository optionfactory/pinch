# Pinch

**The Executable Workflow Manifest & Process Supervisor**

Pinch is a terminal-based process supervisor and developer workflow runner.

Most software projects juggle a scattered collection of scripts, Makefile targets, and ad-hoc terminal tabs to run their local development environment. Pinch solves this with an executable workflow manifest (`pinch.yaml`). A single file supervises your local development background daemons, Docker namespaces, and interactive TUI tasks.

## A Single Source of Truth for Development Workflows

Pinch creates a single source of truth that keeps your development environment aligned:

* **Local Execution & Workflows:** Zero-friction runtime supervision managing multi-process TUIs, Docker namespaces, file watching, and log tailing in one terminal command (`pinch tui`).
* **Declarative Configuration:** Processes, Docker networks, and dashboard layouts declared once and validated via JSON Schema in your IDE.

Because `pinch.yaml` powers the daily development environment, the manifest never goes stale, if the manifest breaks, local development breaks.

> **Governance & compliance:** Project metadata such as service tiers, lifecycle status, and regulatory applicability (DORA, NIS2, CRA, EU AI Act, GDPR, Garante AdS) is tracked in a separate `elaine.yaml` manifest owned by [elaine](https://github.com/optionfactory/elaine), which aggregates and audits it across your organization's repositories.

## Zero-Config IDE Linting

To enable autocompletion and schema validation in VS Code, Neovim (`yamlls`), or JetBrains without any IDE configuration, add this comment as the first line of your `pinch.yaml`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/optionfactory/pinch/refs/heads/master/schema/pinch-v1.schema.json
```

## Security & Trust Model

Pinch is a local process supervisor and developer workflow runner. To evaluate Pinch securely, its trust model and execution boundaries must be understood:

### 1. The User is the Security Boundary
Pinch runs locally under the permissions of the invoking user. It does not introduce an elevated daemon, network service, or multi-tenant boundary. If a user executes `pinch` on their workstation or in a CI/CD pipeline, processes spawn with that user's existing OS privileges and environment variables.

### 2. Treat `pinch.yaml` as Executable Code
A `pinch.yaml` manifest is explicitly designed to execute arbitrary shell commands, spawn Docker containers, and interact with local namespaces.
* **Never execute an untrusted `pinch.yaml` file.** Treat manifests with the exact same security posture as a `Makefile`, `Dockerfile`, `docker-compose.yml`, or `.sh` script.
* **Variable Expansion:** String interpolation (`{{var_name}}`) in commands, environment overrides, and CLI flags is intended to construct dynamic commands. Supplying untrusted user input into variable overrides without external sanitization is outside the intended threat model.

### 3. Docker & Privilege Passthrough
When using `type: "docker"` or `type: "docker-intrude"`, Pinch passes configured flags (`opts`, `args`, `--privileged`, `--net=host`, etc.) directly to the Docker daemon. Pinch does not block or deny-list valid Docker flags, as local development and namespace inspection frequently require host-level access. Security boundaries for container execution must be enforced at the Docker daemon or OS level.

## Installation

### 1. Pre-built Binaries (Linux)
Download the latest statically-linked musl executable directly from the GitHub Releases page, or install it via `curl`:

```bash
curl -sSL \
  https://github.com/optionfactory/pinch/releases/latest/download/pinch-linux-amd64-musl \
  | sudo tee /usr/local/bin/pinch > /dev/null \
  && sudo chmod +x /usr/local/bin/pinch
```

> **Note:** Using processes configured with `type: "docker-intrude"` requires [`docker-intrude`](https://github.com/optionfactory/docker-heist) to be installed and accessible in your system's `PATH`.

> **Note:** Adding `remap_ids` to any process (see below) runs it through [`docker-bluff`](https://github.com/optionfactory/docker-heist), which must then be installed and accessible in your system's `PATH`.

> **Note:** When it runs privileged, `docker-bluff` resolves the program it launches against a fixed secure `PATH` (`/usr/local/bin`, `/usr/bin`, `/bin`, ...), **not** your shell's `PATH`. So a bare `cmd: "mvn"` may resolve to a different binary — or not be found — if it lives somewhere like `~/.local/bin`. Give an absolute path, or run it via `shell: bash` (the shell then resolves the inner command against your inherited `PATH`).

### 2. Build from Source
Ensure you have the Rust toolchain installed, then clone the repository and build:

```bash
git clone https://github.com/optionfactory/pinch
cd pinch
make build-release install
```

## Usage Overview

Pinch uses a structured subcommand hierarchy and global option flags. Global flags can be passed anywhere in your command string.

```bash
pinch [OPTIONS] <COMMAND>
```

### Global Options

| Flag | Argument | Default | Description |
| :--- | :--- | :--- | :--- |
| `-c, --config` | `<FILE>` | `pinch.yaml` | Path to the configuration file (`PINCH_CONFIG` env var supported). |
| `-o, --override` | `<KEY:VAL>` | *None* | Override a configuration variable (repeatable). |
| `-h, --help` | *None* | *None* | Print help information. |
| `-V, --version` | *None* | *None* | Print version information. |

### Supervisor Mode (`tui`)

Launch the full TUI supervisor to manage your processes. If no subcommand is specified, `tui` is executed by default:

```bash
# Run using the default pinch.yaml in the current directory
pinch
pinch tui

# Run using a specific configuration file
pinch -c custom.yaml tui

# Override variables on the fly while launching the TUI
pinch -o env:staging -o target:10.0.0.1 tui
```

## CLI Command Reference

You can interact with specific processes, inspect configuration variables, or manage Docker networks and images directly from your shell without launching the full TUI dashboard. **Commands can be abbreviated**

```text
pinch
├── processes
│   ├── ls (list)             List all available process names
│   ├── show [NAME]           Show the command for a process (or all processes if omitted)
│   └── run <NAME> [-b]       Execute a process directly in foreground or background
├── configuration
│   ├── init                  Generate a default pinch.yaml file
│   ├── show                  Print the parsed, variable-expanded configuration
│   └── var [NAME]            Inspect resolved configuration variables
├── networks
│   ├── ls (list)             List all Docker networks defined in the config
│   ├── show [NAME]           Show the 'docker network create' command
│   └── create [NAME]         Create a Docker network (or all defined networks)
├── containers
│   └── ls (list)             List all unique Docker images used in the config
└── completion <SHELL>        Generate shell completion scripts (bash, zsh, fish)
```

### Process Management (`pinch proc`)

* **`pinch processes ls [-f, --format <FORMAT>]`**: Lists all available process names from the configuration.
* **`pinch processes show [NAME] [-f, --format <FORMAT>]`**: Prints the exact command associated with a process name. If `NAME` is omitted, lists all processes in a map format.
* **`pinch processes run <NAME> [-b, --background]`**: Runs the command associated with the name directly in the foreground or background instead of launching the TUI.
  * `-b, --background`: Spawns the process detached. For processes defined with `type: "docker"`, this automatically runs the container detached (`-d`) instead of interactive (`-ti`).

### Configuration Inspection (`pinch conf`)

* **`pinch configuration init`**: Generates a default `pinch.yaml` file in the current directory.
* **`pinch configuration show [-f, --format <FORMAT>]`**: Shows the fully parsed and variable-expanded configuration.
* **`pinch configuration var [NAME] [-f, --format <FORMAT>]`**: Inspects resolved configuration variables. If `NAME` is provided, prints only that variable's value; otherwise, prints all variables.

### Docker Networks (`pinch net`)

* **`pinch networks ls [-f, --format <FORMAT>]`**: Lists all Docker networks defined in the configuration.
* **`pinch networks show [NAME] [-f, --format <FORMAT>]`**: Shows the `docker network create` command for a specific network (or all if omitted).
* **`pinch networks create [NAME]`**: Creates a specific Docker network (or all if omitted) without starting any processes.

### Containers (`pinch cont`)

* **`pinch containers ls [-f, --format <FORMAT>]`**: Lists all unique, variable-resolved Docker images used by `type: "docker"` processes in the configuration.

```bash
# Example: Pre-pull all required Docker images before starting the supervisor
pinch container ls | xargs -r -n1 docker pull
```

### Shell Completions (`pinch completion`)

* **`pinch completion <SHELL>`**: Supports `bash`, `zsh`, and `fish`.

```bash
# Example: Load completions in bash
source <(pinch completion bash)
```

## Multi-Format Inspection (`-f, --format`)

The `process show`, `process ls`, `config show`, `config var`, `net show`, `net ls`, and `container ls` commands support multiple output formats via `-f, --format <FORMAT>`.

| Format | Output Style | Best Suited For |
| :--- | :--- | :--- |
| **`raw`** | Unquoted plain text or tab-separated pairs | Shell pipelines (`cut`, `awk`, `xargs`, `while read`) |
| **`yaml`** | Clean YAML formatting | Human reading, configuration auditing |
| **`json`** | Structured JSON | Automation and `jq` integration |
| **`properties`** | Standard `key=value` lines | Environment file sourcing (`export $(pinch config var -f properties)`) |

> **Smart Defaults:**
> * **`ls` subcommands** default to **`raw`** (flat line-by-line output) for easy shell composition.
> * **`show [ITEM]`** defaults to **`raw`** when querying a single item (`pinch config var target`), or **`yaml`** when inspecting all items (`pinch config var`).
> * **Structural commands** (`config show`) default to **`yaml`**.


## Variable Overrides & Precedence

Variables defined as `{{var_name}}` inside your YAML file are resolved using a strict 4-layer precedence hierarchy (highest to lowest priority):

| Priority | Source | Description | Example |
| :--- | :--- | :--- | :--- |
| **1 (Highest)** | **CLI `-o, --override` flags** | Explicit runtime overrides | `pinch -o env:prod` |
| **2** | **YAML `vars:` block** | Project defaults defined in `pinch.yaml` | `env: "dev"` |
| **3 (Lowest)** | **Built-in Variables** | System path context | `{{pwd}}`, `{{user}}`, `{{home}}` |

### Expansion Semantics

Substitution is **textual** and happens *before* the command string is parsed, so a value is spliced verbatim into the surrounding text (like a Makefile variable or an unquoted `$VAR` in a shell):

* A value containing spaces produces several arguments: `flags: "-c 10"` in `ping {{ target }} {{ flags }}` yields `ping 8.8.8.8 -c 10`.
* To keep a value with spaces as a single argument, quote the placeholder in the template: `ls "{{ dir }}"`.
* Quotes inside a value are interpreted by the command parser exactly as if written inline, e.g. `opts: "-Dfoo='a b'"` used as `java {{ opts }}` passes a single `-Dfoo=a b` argument.
* Unknown placeholders are left untouched.

The same textual rule applies to every other expanded field (`image`, `cwd`, `link`, mount `source`/`target`, `env` values, ...), where the value is used as-is with no parsing at all.


```bash
# Check the resolved value of a variable
pinch config var target

# Override a variable on the fly
pinch process run simple-ping -o target:1.1.1.1 -o flags:"-c 4"
```

## Keyboard Shortcuts

Pinch relies heavily on keyboard navigation. Behavior adapts automatically depending on whether you are managing the grid, viewing global logs, or interacting with a focused TUI application.

### Global Navigation

| Keybinding | Action |
| :--- | :--- |
| `Ctrl+Q` | Quit Pinch and terminate all child processes. |
| `Ctrl+A` | Toggle the full-screen "Combined Logs" view. |
| `Tab` | Cycle focus to the next pane. |

### Focused Pane Actions (Grid Mode)

| Keybinding | Action |
| :--- | :--- |
| `s` | Start or Stop the focused process. |
| `r` | Restart the focused process. |
| `z` | Toggle Zoom (fullscreen) for the focused pane. |
| `w` | Toggle line wrap (Log mode only). |
| `Ctrl+L` | Clear the log buffer (Log mode only). |
| `Up` / `k` | Scroll logs up. |
| `Down` / `j` | Scroll logs down. |
| `Left` / `h` | Scroll logs left (when line wrap is disabled). |
| `Right` / `l` | Scroll logs right (when line wrap is disabled). |
| `PageUp` / `PageDown` | Scroll logs by 10 lines. |
| `Enter` | Jump to the bottom of the logs (tail), OR focus the TUI (if in TUI mode). |

### TUI Mode (Interactive Process Focus)

If a process is configured with `mode: "tui"`, pressing `Enter` attaches your keyboard directly to that process.
* While focused, all keystrokes are forwarded directly to the underlying application (`top`, `vim`, `htop`, etc.).
* Press **`Ctrl+X`** to detach your keyboard from the TUI and return to Grid Navigation.

### Combined Logs View (`Ctrl+A`)

| Keybinding | Action |
| :--- | :--- |
| `p` | Toggle process name prefixes on/off. |
| `Up` / `k` | Scroll combined logs up. |
| `Down` / `j` | Scroll combined logs down. |
| `PageUp` / `PageDown` | Scroll combined logs by 10 lines. |
| `Enter` | Jump to the bottom of the combined logs (tail). |

### Mouse Support

* **Focus:** Click anywhere on a pane to focus it.
* **Scroll:** Mouse wheel scrolls up and down through logs.
* **Header Buttons:** Click the bracketed indicators in a pane's title bar to trigger actions:
  * `[▶]` / `[■]`: Start / Stop (Green / Red)
  * `[↺]`: Restart (Orange)
  * `[↩]`: Toggle Wrap / Pending Auto-Restart (Cyan / Purple)
  * `[⤢]`: Toggle Zoom (Purple)
  * `[↗]`: Open Link (Electric Blue, visible when `link:` is configured)

---

## Configuration (`pinch.yaml`)

Configuration is defined in YAML. You can define global variables, default behaviors, individual processes, and how they should be laid out on the screen.

### Global Configuration (Root Level)

| Setting | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `schema_version` | Integer | *Required* | Explicit manifest schema version (must be `1`). |
| `name` | String | *Required* | Name of the project. |
| `vars` | Map | `{}` | Custom variables (e.g., `env: "dev"`). Built-ins: `{{pwd}}`, `{{user}}`, `{{home}}`. |
| `logs_max_size` | Integer | *None* | Maximum number of log lines to retain in memory per pane. |
| `shell` | Enum | *None* | If present, executes commands using the specified shell `-c` option globally (`bash`, `zsh`, `fish`). |
| `auto_start` | Boolean | `true` | Whether processes start automatically on launch. |
| `auto_restart` | Boolean | `true` | Whether processes restart automatically if they exit. |
| `grace_period` | Integer | `3000` | Delay in milliseconds before auto-restarting a process. |
| `watch_settle_time_ms` | Integer | `800` | Debounce delay in milliseconds when watching files for changes. |

### Docker Networks

You can define Docker networks in the `docker_networks` block at the root of your configuration. If only **one** network is defined, any process specifying a `docker-intrude` run type will default to it automatically.

```yaml
docker_networks:
  # Simple format (String maps to the subnet)
  simple_net_{{env}}: "172.18.0.1/24"
  # Detailed format (Object with subnet and extra `docker network create` flags,
  # a shell-style string like a process `opts`)
  advanced_net:
    subnet: "172.19.0.1/24"
    args: >
      --ipv6
      --opt com.docker.network.bridge.enable_icc=false
```

### Processes

Each item under `processes` defines a process to supervise:
* `name`: The name of the process.
* `run`: How the process is executed. Supports shorthand strings or detailed objects:
  * **Shorthand (`string`)**: Runs a local command (defaults to `type: "process"`).
  * **Process (`type: "process"`)**:
    * `cmd`: The command string to execute.
    * `shell`: Optional shell override (`bash`, `zsh`, `fish`). Runs the command using `shell -c`.
    * `remap_ids`: Id swaps applied via [`docker-bluff`](https://github.com/optionfactory/docker-heist), one per `--id` (e.g. `me:0`, `u:0:33`, `g:0:33`; `me` resolves to the invoking user's uid/gid). Setting it runs the command under docker-bluff so a container-style UID and the host each see the mounted files as their own (no `chown -R`). Any `shell` wrapping is preserved.
    * `remap_paths`: Directories to remap in place, one per `--map` (`SRC[:DST]`, supports variable expansion). Requires `remap_ids`.
   * **Docker (`type: "docker"`)**: Mirrors the `container` block of `optionfactory.services.bundle`.
     * `engine`: `docker` (default) or `podman`.
     * `name`: Container name passed as `--name` (defaults to the enclosing process name).
     * `image`: The container image to run.
     * `network`, `ip`: Rendered as `--network`/`--ip`; empty values are ignored (enabling conditionals that yield empty strings). If `network` is omitted and exactly one network is defined in `docker_networks`, that network is used automatically (same as `docker-intrude`).
     * `env`: A `KEY: value` mapping, rendered as `--env`.
     * `publish`: A list rendered as `-p`; empty entries are ignored.
     * `mounts`: A list rendered as `--mount type=<type>,source=<source>,target=<target>[,readonly][,<opts>]`. Each entry supports `type` (default `bind`), `source` (mandatory), `target` (defaults to `source`), `readonly` (default `true`), and `opts` (comma-separated extra mount options appended verbatim, e.g. `bind-propagation=rshared`).
     * `volumes`: A list rendered as `--volume`; empty entries are ignored.
     * `opts`: Raw flags appended verbatim after the rendered ones (e.g. `--name`, `--privileged`).
     * `args`: Arguments passed to the container entrypoint.
     * `remap_ids`: Id swaps applied via [`docker-bluff`](https://github.com/optionfactory/docker-heist), one per `--id` (e.g. `me:0`, `u:0:33`, `g:0:33`; `me` resolves to the invoking user's uid/gid). When set, the `docker run` is launched under docker-bluff, which idmaps its own `-v`/`--mount` bind sources so files stay owned correctly on both sides. (There is **no** `remap_paths` for `type: "docker"` — docker-bluff discovers the run's mounts itself.)
  * **Docker Intrude (`type: "docker-intrude"`)**:
    * `ip`: The target IP address in the network namespace.
    * `network`: (Optional if only one network is defined in `docker_networks`) The Docker network name.
    * `cmd`: The command to execute inside the namespace.
    * `shell`: Optional shell override (`bash`, `zsh`, `fish`). Runs the command using `shell -c`.
    * `remap_ids`: Id swaps applied via [`docker-bluff`](https://github.com/optionfactory/docker-heist), one per `--id` (e.g. `me:0`, `u:0:33`, `g:0:33`; `me` resolves to the invoking user's uid/gid). Setting it runs the command under docker-bluff so a container-style UID and the host each see the mounted files as their own (no `chown -R`). Any `shell` wrapping is preserved.
    * `remap_paths`: Directories to remap in place, one per `--map` (`SRC[:DST]`, supports variable expansion). Requires `remap_ids`.
* `mode`: Either `log` (default) or `tui` (allocates a PTY for interactive terminal apps).
* `cwd`: Working directory (supports variables like `{{pwd}}`).
* `link`: An optional web URL or link associated with this process.
* `watch`: A list of file paths. If these files change, a running process is restarted and a crashed one is started again; a process you stopped manually stays stopped.
* `auto_start`, `auto_restart`, `grace_period`, `watch_settle_time_ms`: Overrides global defaults for this specific process.

#### Full Example Configuration

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/optionfactory/pinch/refs/heads/master/schema/pinch-v1.schema.json
schema_version: 1
name: "Fraud-Detection-Service"
docker_networks:
  hi: "172.18.23.0/24"
vars:
  target: "8.8.8.8"
  flags: "-c 10"
processes:
  - name: "simple-ping"
    run: "ping {{ target }} {{ flags }}"
  - name: "system-monitor"
    mode: "tui"
    run:
      type: "process"
      cmd: "top"
  - name: "nginx"
    run:
      type: "docker"
      image: "nginx:alpine"
      network: "hi"
      ip: "172.18.23.10"
      env:
        TZ: "Europe/Rome"
      publish:
        - "0.0.0.0:8888:8888"
      mounts:
        - source: "/opt/myapp/nginx/nginx.conf"
          target: "/etc/nginx/nginx.conf"
      args: "nginx -g 'daemon off;'"
  - name: "google-ping"
    link: "https://www.google.com"
    run:
      type: "docker-intrude"
      ip: "172.18.23.100"
      network: "hi"
      cmd: "ping {{ target }} {{ flags }}"
  - name: "build"
    run:
      # remap_ids/remap_paths run this under docker-bluff so build artifacts stay owned by you
      type: "process"
      cmd: "make package"
      remap_ids:
        - "me:0"
      remap_paths:
        - "{{ pwd }}"
```

## Layout Engine

Pinch uses a **recursive, edge-carving** layout system (`LayoutNode`). Top-level nodes carve percentage slices off screen boundaries, while child nodes recursively split allocated regions into sub-panes.

### How Layout Resolution Works
1. **Canvas Initialization:** Pinch starts with the full terminal grid (`100% width x 100% height`).
2. **Sequential Edge Carving:** Root nodes specifying an `edge` (`left`, `right`, `top`, or `bottom`) sequentially slice percentages off the remaining outer screen area.
3. **Recursive Sub-Splitting:** Nodes with `items` split their assigned area horizontally or vertically across child nodes.
4. **Automatic Sizing:** When child nodes omit `size`, any unallocated space is automatically divided equally among unsized siblings.
5. **Unassigned Panes:** Any process not explicitly referenced in the layout is routed to the node marked `unassigned: true` (or distributed into leftover center space).

### Layout Node Schema

| Field | Type | Description |
| :--- | :--- | :--- |
| `edge` | `top` \| `bottom` \| `left` \| `right` | Carves a percentage slice off the remaining outer terminal bounds. |
| `size` | `integer` (0–100) | Percentage of available space to allocate. Automatically distributed among unsized siblings if omitted. |
| `direction` | `horizontal` \| `vertical` | Split axis for child nodes. Defaults to perpendicular orientation derived from `edge`. |
| `name` | `string` | Target process `name` or `"combined-logs"`. |
| `unassigned` | `boolean` | Designates the node as the fallback container for all unassigned process panes. |
| `items` | `list` | Nested list of child `LayoutNode` elements. |

### Layout Rules & Resolution Behaviors

Layout structures are validated via JSON Schema in your IDE while providing automatic fallbacks in the TUI renderer:

**1. Node Variant Rules**
IDE schema validation enforces three clean structural node types via `oneOf`:
* **Named Leaf Node:** Specifies `name` (e.g., `name: "api-server"` or `name: "combined-logs"`). Must not contain `unassigned` or `items`.
* **Unassigned Leaf Node:** Specifies `unassigned: true`. Must not contain `name` or `items`.
* **Branch Node:** Specifies `items` containing a list of sub-nodes. Must not contain `name` or `unassigned`.

**2. Size & Allocation Limits**
* **Percentage Bounds:** Explicit `size` values are expected to be integers between `0` and `100`.
* **Automatic Auto-Sizing:** If sibling nodes omit `size`, any remaining unallocated percentage after explicit `size` definitions is divided equally among them.

**3. Fallback Container Resolution**
* **Single Unassigned Block:** `unassigned: true` should be specified at most once per layout. If multiple unassigned containers are declared, the runtime engine captures unassigned processes into the first declared container.
* **Implicit Center Grid:** If `unassigned: true` is omitted entirely, any unallocated process panes automatically populate whatever leftover center space remains after edge carving.

**4. Direction & Orientation Defaults**
* Top-level nodes specifying `edge: left` or `edge: right` default their child split `direction` to `vertical`.
* Top-level nodes specifying `edge: top` or `edge: bottom` default their child split `direction` to `horizontal`.
* Child branch nodes inside nested `items` inherit their parent's container bounds and split perpendicular to sibling flow unless an explicit `direction` is defined.

---

### Visual Step-by-Step Example

Given this configuration:

```yaml
layout:
  # Step 1: Carve 30% from the LEFT edge (vertical stack of two panes)
  - edge: "left"
    size: 30
    direction: "vertical"
    items:
      - name: "system-monitor"
        size: 50
      - name: "cpu-mem-stats"
        size: 50

  # Step 2: Carve 25% from the BOTTOM edge of remaining space
  - edge: "bottom"
    size: 25
    direction: "horizontal"
    items:
      - name: "combined-logs"

  # Step 3: Unassigned processes ("ping", "disk-usage") fill the remaining center
```

#### Resulting Terminal Grid

```text
+-------------------+-----------------------------------+
|                   |                                   |
|  System Monitor   |            Network Ping           |
| (30% left / top)  |         (Unassigned Center)       |
|                   |                                   |
|                   |-----------------------------------|
|                   |                                   |
|-------------------|             Disk Usage            |
|                   |         (Unassigned Center)       |
|  CPU & Mem Stats  |                                   |
| (30% left / btm)  |-----------------------------------|
|                   |            Combined Logs          |
|                   |   (25% bottom of remaining space) |
|                   |                                   |
+-------------------+-----------------------------------+
```

### Comprehensive Layout Example

```yaml
schema_version: 1
name: "MyProject"
processes:
  - name: "system-monitor"
    run: "top"
    mode: "tui"
  - name: "backend-api"
    run: "vmstat 1"
  - name: "frontend-ui"
    run: "vmstat 1"
  - name: "database"
    run: "vmstat 1"
  - name: "cache"
    run: "vmstat 1"
layout:
  # 1. Carve out 35% from the left edge
  - edge: "left"
    size: 35
    direction: "vertical"
    items:
      - name: "system-monitor"
        size: 40
      # recursive sub-split: Divide the remaining 60% vertical area horizontally into sub-panes
      - direction: "horizontal"
        items:
          - name: "database"
          - name: "cache" # Omitting `size` automatically splits space 50/50 between database & cache
  # 2. Carve out 30% from the bottom of remaining space for logs and unassigned processes
  - edge: "bottom"
    size: 30
    direction: "horizontal"
    items:
      - name: "combined-logs"
        size: 50
      - unassigned: true # "backend-api" and "frontend-ui" are automatically placed side-by-side here
        size: 50
```