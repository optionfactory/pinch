# Pinch

**Pinch** is a terminal-based task manager and process supervisor. 

It allows you to orchestrate, monitor, and interact with project tasks, background services, and TUI applications from a single terminal window. It handles basic process lifecycle management (start, stop, restart, auto-restart on file changes) and provides a customizable grid and edge-based layout system.

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
| `-v, --var` | `<KEY:VAL>` | *None* | Set or override a configuration variable (repeatable). |
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
pinch -v env:staging -v target:10.0.0.1 tui
```

---

## CLI Command Reference

You can interact with specific processes, inspect configuration variables, or manage Docker networks and images directly from your shell without launching the full TUI dashboard.

```
pinch
 ├── process
 │    ├── ls (list)             List all available process titles
 │    ├── show [TITLE]          Print the command for a process (or all processes)
 │    └── run <TITLE> [-b]      Execute a process directly in foreground or background
 ├── config
 │    ├── init                  Generate a default pinch.yaml file
 │    ├── show                  Print the parsed, variable-expanded configuration
 │    └── var [NAME]            Inspect resolved configuration variables
 ├── net
 │    ├── ls (list)             List all Docker networks defined in the config
 │    ├── show [NAME]           Show the 'docker network create' command
 │    └── create [NAME]         Create a Docker network (or all defined networks)
 ├── image
 │    └── ls (list)             List all unique Docker images used in the config
 └── completion <SHELL>         Generate shell completion scripts (bash, zsh, fish)
```

### Process Management (`pinch process`)
* **`pinch process ls`** (alias: `list`): Lists all available process titles from the configuration.
* **`pinch process show [TITLE] [-f, --format <FORMAT>]`**: Prints the exact command associated with a process title. If `TITLE` is omitted, lists all processes.
* **`pinch process run <TITLE> [-b, --background]`**: Runs the command associated with the title directly in the foreground or background instead of launching the TUI.
  * `-b, --background`: Spawns the process detached. For processes defined with `type: "docker"`, this automatically runs the container detached (`-d`) instead of interactive (`-ti`).

### Configuration Inspection (`pinch config`)
* **`pinch config init`**: Generates a default `pinch.yaml` file in the current directory.
* **`pinch config show [-f, --format <FORMAT>]`**: Shows the fully parsed and variable-expanded configuration.
* **`pinch config var [NAME] [-f, --format <FORMAT>]`**: Inspects resolved configuration variables. If `NAME` is provided, prints only that variable's value; otherwise, prints all variables.

### Docker Networks (`pinch net`)
* **`pinch net ls`** (alias: `list`): Lists all Docker networks defined in the configuration.
* **`pinch net show [NAME] [-f, --format <FORMAT>]`**: Shows the `docker network create` command for a specific network (or all if omitted).
* **`pinch net create [NAME]`**: Creates a specific Docker network (or all if omitted) without starting any processes.

### Docker Images (`pinch image`)
* **`pinch image ls [-f, --format <FORMAT>]`** (alias: `list`): Lists all unique, variable-resolved Docker images used by `type: "docker"` processes in the configuration.

```bash
# Example: Pre-pull all required Docker images before starting the supervisor
pinch image ls | xargs -r -n1 docker pull
```

### Shell Completions (`pinch completion`)
* **`pinch completion <SHELL>`**: Supports `bash`, `zsh`, and `fish`.

```bash
# Example: Load completions in Zsh
source <(pinch completion zsh)
```

---

## Multi-Format Inspection (`-f, --format`)

The `process show`, `config show`, `config var`, `net show`, and `image ls` commands support multiple output formats via `-f, --format <FORMAT>`.

| Format | Output Style | Best Suited For |
| :--- | :--- | :--- |
| **`raw`** | Unquoted plain text or tab-separated pairs | Shell pipelines (`cut`, `awk`, `xargs`, `while read`) |
| **`yaml`** | Clean YAML formatting | Human reading, configuration auditing |
| **`json`** | Structured JSON | Automation and `jq` integration |
| **`properties`** | Standard `key=value` lines | Environment file sourcing |

> **Smart Defaults:**  
> * When inspecting a **single item** (e.g., `pinch config var target` or `pinch process show "Backend"`) or a simple list (`pinch image ls`), the output defaults to **`raw`** so it can be piped directly into scripts (`IP=$(pinch config var target)`).  
> * When inspecting **all mapped items** (e.g., `pinch config var` or `pinch process show`), the output defaults to **`yaml`** with keys sorted alphabetically.

---

## Variable Overrides & Precedence

Variables defined as `{{var_name}}` inside your YAML file are resolved using a strict 4-layer precedence hierarchy (highest to lowest priority):

| Priority | Source | Description | Example |
| :--- | :--- | :--- | :--- |
| **1 (Highest)** | **CLI `-v, --var` flags** | Explicit runtime overrides | `pinch -v env:prod` |
| **2** | **OS Environment Variables** | Current shell context | `export env=staging` |
| **3** | **YAML `vars:` block** | Project defaults defined in `pinch.yaml` | `env: "dev"` |
| **4 (Lowest)** | **Built-in Variables** | System path context | `{{pwd}}`, `{{user}}`, `{{home}}` |

Because Pinch automatically falls back to your OS environment variables, you do not need to pass `-v` for variables already exported in your shell.

```bash
# Check the resolved value of a variable
pinch config var target

# Override a variable on the fly
pinch process run "Simple Ping" -v target:1.1.1.1 -v flags:"-c 4"
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
| `title` | String | *Required* | The title displayed at the top of the dashboard. |
| `vars` | Map | `{}` | Custom variables (e.g., `env: "dev"`). Built-ins: `{{pwd}}`, `{{user}}`, `{{home}}`. |
| `logs_max_size` | Integer | *None* | Maximum number of log lines to retain in memory per pane. |
| `shell` | Boolean | `false` | If true, executes shorthand commands via `bash -c`. |
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
  
  # Detailed format (Object maps to subnet and custom args as a list)
  advanced_net:
    subnet: "172.19.0.1/24"
    args: 
      - "--ipv6"
      - "--opt"
      - "com.docker.network.bridge.enable_icc=false"
```

### Processes
Each item under `processes` defines a process to supervise. You must define a `run` configuration, which can either be a shorthand command string or a structured object using `cmd:` and `opts:`.

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

#### Example Process Configuration
```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/optionfactory/pinch/refs/heads/master/schema/pinch-v1.schema.json
project:
  name: "Development Dashboard"
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

Pinch uses a progressive, edge-carving layout system. Items defined in the `layout` array sequentially carve out space from the edges of the terminal. Any processes not explicitly listed in the layout automatically fill whatever space is left in the center grid, *unless* you designate a specific block or split to hold them using `unassigned: true`. You can also use the special title `"Combined Logs"` to embed the global log tail directly into your dashboard.

### Visualizing Edge-Carving

```
+-------------------------------------------------------+
|  System Monitor (60%)          |                      |
|--------------------------------|   Unassigned Grid    |
|  CPU & Mem Stats (40%)         |    (Center Area)     |
|  [Edge: Left, Size: 35%]       |                      |
+-------------------------------------------------------+
|  Combined Logs (50%)      |  Simulated Service (50%)  |
|  [Edge: Bottom, Size: 30%]                            |
+-------------------------------------------------------+
```

### Layout Options
* `edge`: Which side to carve from (`left`, `right`, `top`, `bottom`).
* `size_percentage`: How much of the currently available space to take (0–100).
* `direction`: How to arrange sub-splits (`horizontal` or `vertical`).
* `splits`: An array of sub-panes to place inside this carved edge.
* `unassigned`: (Optional boolean) If `true`, all remaining unassigned processes are routed to this specific block or split instead of the default center area.

### Example Layout Configuration
```yaml
project:
  name: "Development Dashboard"
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
  # 1. Carve out the left 35% of the screen, split vertically
  - edge: "left"
    size_percentage: 35
    direction: "vertical"
    splits:
      - title: "System Monitor"
        size_percentage: 50
      - title: "Database"
        size_percentage: 50
  # 2. Carve out the bottom 30% of the remaining space for global logs and unassigned processes
  - edge: "bottom"
    size_percentage: 30
    direction: "horizontal"
    splits:
      - title: "Combined Logs"
        size_percentage: 50
      - unassigned: true # "Backend API" and "Frontend UI" will be placed here automatically
        size_percentage: 50
```