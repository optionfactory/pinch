# Pinch

A TUI-based multi-process supervisor. 

Pinch is a terminal interface for running, monitoring, and managing multiple background processes (like dev servers, file watchers, or databases) in  pseudo-terminals.

## Features

* **Grid & Zoom UI:** View multiple processes in a grid or zoom into a single pane.
* **Combined Logs:** Merge all process logs into a single chronological view to track cross-service output.
* **Process Management:** Supports auto-start, auto-restart, and graceful shutdowns.
* **Terminal Emulation:** Native PTY support 
* **Mouse & Keyboard:** Navigable via keyboard shortcuts or mouse.

## Installation

**Download Pre-built Binaries (Linux)**
You can download the latest statically-linked `musl` executable directly from the [GitHub Releases page](https://github.com/optionfactory/pinch/releases). 

After downloading, rename the binary and make it executable:
```bash
mv pinch-vX.Y.Z-x86_64-unknown-linux-musl pinch
chmod +x pinch
./pinch
```

## Build from Source
Ensure you have Rust installed, then clone the repository and build:

```bash
git clone [https://github.com/yourusername/pinch.git](https://github.com/yourusername/pinch.git)
cd pinch
cargo build --release
```

_Note_: Pinch looks for a pinch.yaml file in your current directory by default. You can also specify a path: pinch ./config.yaml

## Configuration
Pinch uses a YAML configuration file.

```yaml
# pinch.yaml
title: "My Dev Environment"
logs_max_size: 1000 #optional, default: unlimited
auto_start: true    #optional, default: true
auto_restart: true  #optional, default: true
grace_period: 3000  #optional, default: 3000, milliseconds to wait between restarts
shell: false        #optional, default: false, use `bash -c` to start cmd
vars: 
    anyvar: anyvalue
processes:
  - title: "Frontend"
    cmd: >
        npm run dev -- 
          --port 3000    
    cwd: "./frontend"   #optional, default working directory to use, default: current
    auto_start: true    #optional, default: true
    auto_restart: true  #optional, default: true
    grace_period: 3000  #optional, default: 3000, milliseconds to wait between restarts
    shell: false        #optional, default: false, use `bash -c` to start cmd
  - title: "Backend API"
    cmd: cargo run
  - title: "Tailwind Watcher"
    cmd: >
        npx tailwindcss 
            -i ./src/input.css 
            -o ./dist/output.css 
            --watch
```


## Keybindings

### Global

- `Ctrl+q`: Gracefully stop all processes and exit
- `a`: Toggle the combined Global Logs view
- `Tab`: Cycle focus to the next pane
-  `↕` or `Mouse Wheel`: Scroll logs
- `Enter`: Tail logs (jump to bottom)

### Focused Pane
- `s`: Start / Stop the current process
- `r`: Restart the current process
- `z`: Zoom into the current process
- `w`: Toggle line wrapping
- `h`/`l` or `←`/`→`: Scroll horizontally (when wrapping is off)

### Global Logs
- `p`: Toggle process name prefixes