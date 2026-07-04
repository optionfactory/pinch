# Pinch
Pinch is a terminal-based process supervisor. 
It allows you to run, monitor, and interact with multiple background processes or TUI applications from a single terminal window.

It handles basic process lifecycle management (start, stop, restart, auto-restart on file changes) and provides
a customizable grid and edge-based layout system.

## Installation
Download Pre-built Binaries (Linux) You can download the latest statically-linked musl executable directly from the GitHub Releases page.

or just run:

```bash
curl -sSL \
  https://github.com/optionfactory/pinch/releases/latest/download/pinch-amd64-linux-musl \
  | sudo tee /usr/local/bin/pinch > /dev/null \
  && sudo chmod +x /usr/local/bin/pinch
  
```

Using `docker_ip`, requires [`docker-intrude`](https://github.com/optionfactory/docker-intrude) to be installed.

## Build from Source
Ensure you have Rust installed, then clone the repository and build:

```bash
git clone https://github.com/optionfactory/pinch
cd pinch
make build-release
sudo make install
```

## Usage
`pinch --init`: Generates a default pinch.yaml in the current directory.

`pinch`: Runs the supervisor using pinch.yaml in the current directory.

`pinch custom.yaml`: Runs the supervisor using a specific configuration file.

## Keyboard Shortcuts

Pinch relies heavily on keyboard navigation. Behavior changes depending on whether you are looking at the global logs, managing the grid, or interacting with a focused TUI application.

### Global Shortcuts

|Keybinding | Action |
| :-        | :- 
| `Ctrl+Q`  | Quit Pinch and terminate all child processes.
| `Ctrl+A`  | Toggle the full-screen "Combined Logs" view.

### Grid Navigation (Default Mode)

When navigating the layout grid, the following shortcuts apply to the currently highlighted/focused pane (indicated by a blue border).

| Keybinding | Action |
| :-        | :- 
| `Tab` |Cycle focus to the next pane.sStart or Stop the focused process.
| `r` | Restart the focused process.
| `z` | Toggle Zoom (fullscreen) for the focused pane.
| `w` |Toggle line wrap (Log mode only).
| `Ctrl+L` | Clear the log buffer (Log mode only).
| `Up` / `k`| Scroll logs up.
| `Down` / `j`| Scroll logs down.
| `Left` / `h`| Scroll logs left (only when wrap is disabled).
| `Right` / `l`| Scroll logs right (only when wrap is disabled).
| `PageUp` / `PageDown`| Scroll logs by 10 lines.
| `Enter` | Jump to the bottom of the logs (tail), OR focus the TUI (if in TUI mode).

### TUI Mode (Focused)
If a process is configured with mode: "tui", pressing `Enter` will "focus" or "attach" your keyboard to that process.

* While focused, all keystrokes are forwarded directly to the underlying application (e.g., top, vim, htop).
* `Ctrl+X`: Detach your keyboard from the TUI and return to Grid Navigation.

### Combined Logs View

When you open the global combined logs (via `Ctrl+A` or by putting it in your layout):

|Keybinding|Action|
| :-        | :- 
| `p` | Toggle process name prefixes on/off.
| `Up` / `k` | Scroll combined logs up.
| `Down` / `j` | Scroll combined logs down.
| `PageUp` / `PageDown` | Scroll combined logs by 10 lines.
| `Enter` | Jump to the bottom of the combined logs (tail).

### Mouse Support

Pinch supports basic mouse interactions:
* Click anywhere on a pane to focus it.
* Scroll Wheel scrolls up and down through logs.
* Clicking Header Buttons: You can click the brackets in a pane's title bar to trigger actions:
  * [▶] / [■]: Start / Stop
  * \[↺]: Restart
  * [W]: Toggle Wrap
  * [Z]: Toggle Zoom
  
## Configuration (`pinch.yaml`)

Configuration is defined in YAML. You can define global variables, default behaviors, individual processes, and how they should be laid out on the screen. 

Many settings can be defined globally at the root level and overridden on a per-process basis.

### Global Configuration (Root Level)
* `title`: The title displayed at the top of the dashboard.
* `vars`: A map of custom variables (e.g., `env: "dev"`). Use `{{var_name}}` to inject them into paths or commands. Built-ins: `{{pwd}}`, `{{user}}`, `{{home}}`.
* `logs_max_size`: Maximum number of log lines to retain in memory per pane.
* `shell`: (Default `false`) If true, runs commands via `bash -c`.
* `auto_start`: (Default `true`) Whether processes start on launch.
* `auto_restart`: (Default `true`) Whether processes restart if they exit.
* `grace_period`: (Default `3000`) Delay in milliseconds before auto-restarting.
* `watch_settle_time_ms`: (Default `800`) Debounce time in ms when watching files for changes.
* `docker_network`: Global docker network to use for `docker-intrude` features.

### Processes

Each item under `processes` defines a command to run.

* `title`: The display name of the process.
* `cmd`: The command to execute.
* `cwd`: Working directory (supports variables like `{{pwd}}`).
* `mode`: Either `log` (default) or `tui` (allocates a PTY for interactive terminal apps).
* `watch`: A list of file paths. If these files change, the process restarts.
* `shell`, `auto_start`, `auto_restart`, `grace_period`, `watch_settle_time_ms`: Overrides the global defaults for this specific process.
* `docker_ip`: If set, Pinch uses `docker-intrude` to run the process inside a specific Docker network namespace (requires `docker_network` to be set globally or locally).

#### Layout Engine

Pinch uses a progressive, edge-carving layout system. Items defined in the `layout` array sequentially carve out space from the edges of the terminal. Any processes not explicitly listed in the layout will automatically fill whatever space is left in the center grid.

You can also use the special title `"Combined Logs"` to embed the global log tail directly into your dashboard.

##### Layout Options*:
`edge`: Which side to carve from (left, right, top, bottom).`size_percentage`: How much of the currently available space to take (0-100).
`direction`: How to arrange sub-splits (horizontal or vertical).
`splits`: An array of sub-panes to place inside this carved edge.

##### Example Layout Configuration:

```yaml
title: "Development Dashboard"
processes:
  - title: "System Monitor"
    cmd: "top"
    mode: "tui"
  - title: "Backend API"
    cmd: "cargo run"
  - title: "Frontend UI"
    cmd: "npm run dev"
  - title: "Database"
    cmd: "docker logs -f pg_db"
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

  # 2. Carve out the bottom 30% of the remaining space for global logs
  - edge: "bottom"
    size_percentage: 30
    title: "Combined Logs"

  # "Backend API" and "Frontend UI" are unassigned and will automatically 
  # share the remaining top-right grid space.
```