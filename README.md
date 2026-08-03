# Pinch
**The Executable Service Manifest & DevSecOps Supervisor**

Pinch is a terminal-based process supervisor, developer workflow runner, and **lightweight "compliance-as-code" catalog**.

Most software projects suffer from a disconnect between execution and governance: traditional process runners know how to execute code but ignore ownership and SLAs, while dedicated compliance portals capture governance metadata but sit in separate systems that developers rarely visit after onboarding.

Pinch solves this with an **executable service manifest** (`pinch.yaml`). The exact same file that supervises your local background daemons, Docker namespaces, and interactive TUI tasks also serves as your repository's authoritative contract for operational tiers, data sensitivity, and European regulatory compliance.

---

## A Single Source of Truth for Cross-Functional Teams

By uniting runtime supervision with **"compliance-as-code"**, Pinch creates a single source of truth that keeps three critical aspects of your software contract aligned without creating metadata rot:

* **Local Execution & Workflows:** Zero-friction runtime supervision—managing multi-process TUIs, Docker namespaces, file watching, and log tailing in one terminal command (`pinch tui`).
* **Operational Standards & SLAs:** Consistent service tiers (`tier-1` to `tier-4`), lifecycle status, environment exposures, and container dependencies enforced across repositories.
* **Regulatory Governance:** European and Italian compliance mandates (DORA, NIS2, CRA, EU AI Act, GDPR, Garante AdS) verifiable directly in CI pipelines (`pinch audit`).

Because `pinch.yaml` powers the daily development environment, architecture and compliance metadata never go stale—if the manifest breaks, local development breaks.

---

## Zero-Config IDE Linting

To enable autocompletion, schema validation, and regulatory applicability tooltips in VS Code, Neovim (`yamlls`), or JetBrains without any IDE configuration, add this comment as the first line of your `pinch.yaml`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/optionfactory/pinch/refs/heads/master/schema/pinch-v1.schema.json
```

---

## Installation

### 1. Pre-built Binaries (Linux)

Download the latest statically-linked musl executable directly from the GitHub Releases page, or install it via `curl`:

```bash
curl -sSL \
  https://github.com/optionfactory/pinch/releases/latest/download/pinch-linux-amd64-musl \
  | sudo tee /usr/local/bin/pinch > /dev/null \
  && sudo chmod +x /usr/local/bin/pinch
```

> **Note:** Using the `docker_ip` configuration requires [`docker-intrude`](https://github.com/optionfactory/docker-intrude) to be installed on your system.

### 2. Build from Source

Ensure you have the Rust toolchain installed, then clone the repository and build:

```bash
git clone https://github.com/optionfactory/pinch
cd pinch
make build-release
sudo make install
```

---

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

---

## CLI Command Reference

You can interact with specific processes, inspect configuration variables, or manage Docker networks and images directly from your shell without launching the full TUI dashboard.

**Commands can be abbreviated**

```text
pinch
  processes
    ls (list)             List all available process titles
    show [TITLE]          Show the command for a process (or all processes if omitted)
    run <TITLE> [-b]      Execute a process directly in foreground or background

  configuration
    init                  Generate a default pinch.yaml file
    show                  Print the parsed, variable-expanded configuration
    var [NAME]            Inspect resolved configuration variables

  networks
    ls (list)             List all Docker networks defined in the config
    show [NAME]           Show the 'docker network create' command
    create [NAME]         Create a Docker network (or all defined networks)

  containers
    ls (list)             List all unique Docker images used in the config

  audit                   Inspect project metadata, compliance rules, and container dependencies

  completion <SHELL>      Generate shell completion scripts (bash, zsh, fish)
```

### Process Management (`pinch proc`)

* **`pinch processes ls [-f, --format <FORMAT>]`**: Lists all available process titles from the configuration.
* **`pinch processes show [TITLE] [-f, --format <FORMAT>]`**: Prints the exact command associated with a process title. If `TITLE` is omitted, lists all processes in a map format.
* **`pinch processes run <TITLE> [-b, --background]`**: Runs the command associated with the title directly in the foreground or background instead of launching the TUI.
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

### Audit (`pinch audit`)

* **`pinch audit [-f, --format <FORMAT>]`**: Prints structured audit metadata combining project identification, governance tiers, regulatory compliance, and resolved container dependencies (defaults to `json`).

### Shell Completions (`pinch completion`)

* **`pinch completion <SHELL>`**: Supports `bash`, `zsh`, and `fish`.

```bash
# Example: Load completions in Zsh
source <(pinch completion zsh)
```

---

## Multi-Format Inspection (`-f, --format`)

The `process show`, `process ls`, `config show`, `config var`, `net show`, `net ls`, `container ls`, and `audit` commands support multiple output formats via `-f, --format <FORMAT>`.

| Format | Output Style | Best Suited For |
| :--- | :--- | :--- |
| **`raw`** | Unquoted plain text or tab-separated pairs | Shell pipelines (`cut`, `awk`, `xargs`, `while read`) |
| **`yaml`** | Clean YAML formatting | Human reading, configuration auditing |
| **`json`** | Structured JSON | Automation and `jq` integration |
| **`properties`** | Standard `key=value` lines | Environment file sourcing (`export $(pinch config var -f properties)`) |

> **Smart Defaults:**
> * **`ls` subcommands** default to **`raw`** (flat line-by-line output) for easy shell composition.
> * **`show [ITEM]`** defaults to **`raw`** when querying a single item (`pinch config var target`), or **`yaml`** when inspecting all items (`pinch config var`).
> * **Structural commands** (`config show`, `project show`) default to **`yaml`**.
> * **`audit`** defaults to **`json`**.

---

## Variable Overrides & Precedence

Variables defined as `{{var_name}}` inside your YAML file are resolved using a strict 4-layer precedence hierarchy (highest to lowest priority):

| Priority | Source | Description | Example |
| :--- | :--- | :--- | :--- |
| **1 (Highest)** | **CLI `-o, --override` flags** | Explicit runtime overrides | `pinch -o env:prod` |
| **2** | **OS Environment Variables** | Current shell context | `export env=staging` |
| **3** | **YAML `vars:` block** | Project defaults defined in `pinch.yaml` | `env: "dev"` |
| **4 (Lowest)** | **Built-in Variables** | System path context | `{{pwd}}`, `{{user}}`, `{{home}}` |

Because Pinch automatically falls back to your OS environment variables, you do not need to pass `-o` for variables already exported in your shell.

```bash
# Check the resolved value of a variable
pinch config var target

# Override a variable on the fly
pinch process run "Simple Ping" -o target:1.1.1.1 -o flags:"-c 4"
```

---

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
  * `[ ]` / `[ ]`: Start / Stop
  * `[ ]`: Restart
  * `[W]`: Toggle Wrap
  * `[Z]`: Toggle Zoom

---

## Configuration (`pinch.yaml`)

Configuration is defined in YAML. You can define global variables, default behaviors, individual processes, and how they should be laid out on the screen.

### Global Configuration (Root Level)

| Setting | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `project` | Object | *Required* | High-level project metadata (`name`, `type`, `tier`, `lifecycle`, `compliance`, `auth`). |
| `vars` | Map | `{}` | Custom variables (e.g., `env: "dev"`). Built-ins: `{{pwd}}`, `{{user}}`, `{{home}}`. |
| `logs_max_size` | Integer | *None* | Maximum number of log lines to retain in memory per pane. |
| `shell` | Boolean | `false` | If true, executes shorthand commands via `bash -c`. |
| `auto_start` | Boolean | `true` | Whether processes start automatically on launch. |
| `auto_restart` | Boolean | `true` | Whether processes restart automatically if they exit. |
| `grace_period` | Integer | `3000` | Delay in milliseconds before auto-restarting a process. |
| `watch_settle_time_ms` | Integer | `800` | Debounce delay in milliseconds when watching files for changes. |

---

### Project Metadata & Governance (`project:`)

The `project` block defines ownership, operational SLAs, security posture, and regulatory compliance rules:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/optionfactory/pinch/refs/heads/master/schema/pinch-v1.schema.json
project:
  name: "Customer Fraud AI Analyzer"
  type: service               # library | service | tool | job
  lifecycle: active           # active | maintenance | deprecated | prototype
  tier: tier-1                # tier-1 (24/7 SLA) -> tier-4 (experimental)
  authentication:
    - jwt
    - mtls
  sensitivity:
    - pii
    - financial
  compliance:
    dora: cif-supported       # EU Digital Operational Resilience Act
    cra: important-class-2    # EU Cyber Resilience Act
    nis2: essential-entity    # EU NIS2 Critical Infrastructure
    aiact: high-risk          # EU AI Act risk classification
    gdpr:
      role: processor
      eu_residency_only: true
    ads:                      # Italian Garante Privacy 'Amministratore di Sistema'
      responsibility: internal
      logging: immutable-12-months
      nomination_executed: true
      latest_audit_date: "2026-02-10"
  environments:
    production:
      exposure: restricted-ip
      domains:
        api.internal.org: managed
```

#### Governance & Regulatory Standards Covered
* **Service Tiers (`tier`):** Maps internal operational priority (`tier-1` critical path down to `tier-4` prototype).
* **Data Sensitivity (`sensitivity`):** Classifies assets handled by the project (`public`, `internal`, `confidential`, `restricted`, `pii`, `spi`, `biometric`, `pci`, `financial`, `phi`).
* **European Regulatory Mappings (`compliance`):**
  * **`dora`:** Tracks whether the project supports a Critical or Important Function (`cif-supported` / `non-critical`) under EU financial resilience rules.
  * **`cra`:** Maps Cyber Resilience Act classes (`default`, `important-class-1`, `important-class-2`, `critical`).
  * **`nis2`:** Non-financial critical infrastructure entities (`essential-entity`, `important-entity`, `out-of-scope`).
  * **`ai_act`:** Enforces EU AI Act risk classifications (`high-risk`, `general-purpose-ai`, `limited-risk`, `minimal-risk`).
  * **`gdpr`:** Explicitly captures legal processing roles (`controller`, `processor`, `sub-processor`) and EU/EEA data residency boundaries.
  * **`ads`:** Italian Data Protection (*Provvedimento Garante AdS*) governance tracking system administrators, immutable access log retention (`immutable-6-months` / `immutable-12-months`), formal appointment letters, and annual audit dates.

---

### Docker Networks

You can define Docker networks in the `docker_networks` block at the root of your configuration. If only **one** network is defined, any process specifying a `docker-intrude` run type will default to it automatically.

```yaml
docker_networks:
  # Simple format (String maps to the subnet)
  simple_net_{{env}}: "172.18.0.1/24"

  # Detailed format (Object maps to subnet and custom args as a list)
  advanced_net:
    subnet: "172.19.0.1/24"
    args: 
      - "--ipv6"
      - "--opt"
      - "com.docker.network.bridge.enable_icc=false"
```

---

### Processes

Each item under `processes` defines a process to supervise:

* `title`: The display name of the process.
* `run`: How the process is executed. Supports shorthand strings or detailed objects:
  * **Shorthand (`string`)**: Runs a local command (defaults to `type: "process"`, `bash: false`).
  * **Process (`type: "process"`)**:
    * `cmd`: The command string to execute.
    * `bash`: (Default `false`) If `true`, runs via `bash -c`.
  * **Docker (`type: "docker"`)**:
    * `image`: The Docker image to run.
    * `opts`: Arguments passed to `docker run --rm` (`--name`, `--network`, `--ip`, etc.).
    * `args`: Arguments passed to the container entrypoint.
  * **Docker Intrude (`type: "docker-intrude"`)**:
    * `ip`: The target IP address in the network namespace.
    * `network`: (Optional if only one network is defined in `docker_networks`) The Docker network name.
    * `cmd`: The command to execute inside the namespace.
    * `bash`: (Default `false`) If `true`, runs via `bash -c`.
* `mode`: Either `log` (default) or `tui` (allocates a PTY for interactive terminal apps).
* `cwd`: Working directory (supports variables like `{{pwd}}`).
* `link`: An optional web URL or link associated with this process.
* `watch`: A list of file paths. If these files change, the process restarts automatically.
* `auto_start`, `auto_restart`, `grace_period`, `watch_settle_time_ms`: Overrides global defaults for this specific process.

#### Full Example Configuration

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/optionfactory/pinch/refs/heads/master/schema/pinch-v1.schema.json
project:
  name: "Fraud-Detection-Service"
  type: service
  lifecycle: active
  tier: tier-1
  authentication:
    - mtls
    - jwt
  sensitivity:
    - pii
    - financial
  compliance:
    dora: cif-supported
    ai_act: high-risk
    ads:
      responsibility: internal
      logging: immutable-12-months
      nomination_executed: true
      latest_audit_date: "2026-02-10"

docker_networks:
  hi: "172.18.23.0/24"

vars:
  target: "8.8.8.8"
  flags: "-c 10"

processes:
  - title: "Simple Ping"
    run: "ping {{ target }} {{ flags }}"
  - title: "System Monitor"
    mode: "tui"
    run:
      type: "process"
      bash: false
      cmd: "top"
  - title: "Nginx Test"
    run:
      type: "docker"
      image: "nginx:alpine"
      opts: >
        --name hi-nginx
        --network hi
        --ip 172.18.23.10
      args: "nginx -g 'daemon off;'"
  - title: "Google DNS (Namespace)"
    link: "https://www.google.com"
    run:
      type: "docker-intrude"
      ip: "172.18.23.100"
      network: "hi"
      cmd: "ping {{ target }} {{ flags }}"
```

---

## Layout Engine

Pinch uses a **progressive, edge-carving** layout system. 

### How Edge-Carving Works

1. **The Starting Canvas:** Pinch begins with a single full-screen rectangle (`100% width x 100% height`).
2. **Sequential Carving:** Each block defined in your `layout` array carves out a slice from one of the outer edges (`left`, `right`, `top`, or `bottom`) of the *currently remaining* screen space.
3. **Sub-splitting:** A carved slice can be divided into sub-panes (`splits`) horizontally or vertically.
4. **Unassigned Area:** Any process *not* explicitly placed in the layout automatically populates whatever space remains in the center (or gets routed to a block with `unassigned: true`).

---

### Visual Step-by-Step

Given this configuration:

```yaml
layout:
  # Step 1: Carve 30% from the LEFT edge (split top/bottom)
  - edge: "left"
    size_percentage: 30
    direction: "vertical"
    splits:
      - title: "System Monitor"
        size_percentage: 50
      - title: "CPU & Mem Stats"
        size_percentage: 50

  # Step 2: Carve 25% from the BOTTOM edge of the REMAINING space
  - edge: "bottom"
    size_percentage: 25
    direction: "horizontal"
    splits:
      - title: "Combined Logs"
        size_percentage: 100

  # Step 3: Unassigned processes ("Network Ping", "Disk Usage") fill the remaining center
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

---

### Layout Configuration Options

* **`edge`**: Which side to carve from (`left`, `right`, `top`, `bottom`).
* **`size_percentage`**: Percentage of the *currently available* screen space to carve out (0 to 100).
* **`direction`**: Orientation for sub-splits (`horizontal` or `vertical`).
* **`splits`**: An array of sub-panes inside this carved slice.
  * **`title`**: Display title matching a process, or `"Combined Logs"` for the global log stream.
  * **`size_percentage`**: Percentage of space within this slice allocated to the sub-pane.
  * **`unassigned`**: (`true`/`false`) If set to `true`, routes all remaining unassigned processes into this sub-pane instead of the default center space.

---

### Full Layout Example

```yaml
project:
  name: "MyProject"
  type: service
processes:
  - title: "System Monitor"
    run: "top"
    mode: "tui"
  - title: "Backend API"
    run: "cargo run"
  - title: "Frontend UI"
    run: "npm run dev"
  - title: "Database"
    run: "docker logs -f pg_db"
layout:
  # 1. Carve out 35% from the left edge, split into System Monitor and Database
  - edge: "left"
    size_percentage: 35
    direction: "vertical"
    splits:
      - title: "System Monitor"
        size_percentage: 50
      - title: "Database"
        size_percentage: 50
  # 2. Carve out 30% from the bottom of the remaining screen for global logs and unassigned processes
  - edge: "bottom"
    size_percentage: 30
    direction: "horizontal"
    splits:
      - title: "Combined Logs"
        size_percentage: 50
      - unassigned: true # "Backend API" and "Frontend UI" are automatically placed side-by-side here
        size_percentage: 50
```