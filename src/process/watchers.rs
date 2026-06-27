use crate::events::PinchEvent;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub fn start_watcher(
    config: &crate::config::PinchConfig,
    tx_watcher: mpsc::Sender<PinchEvent>,
    is_running: Arc<AtomicBool>,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let (watch_tx, mut watch_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = watch_tx.send(event);
                }
            }
        },
        Config::default(),
    )?;

    let mut path_to_pane: Vec<(PathBuf, usize)> = Vec::new();
    let mut pane_settle_times: HashMap<usize, Duration> = HashMap::new();

    for (idx, process) in config.processes.iter().enumerate() {
        pane_settle_times.insert(idx, Duration::from_millis(process.watch_settle_time_ms));

        for path in &process.watch {
            if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                eprintln!("Warning: Failed to watch path {:?} - {}", path, e);
            } else {
                let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
                path_to_pane.push((resolved, idx));
            }
        }
    }

    let handle = tokio::spawn(async move {
        let mut pending_restarts: HashMap<usize, Instant> = HashMap::new();

        while is_running.load(Ordering::SeqCst) {
            match tokio::time::timeout(Duration::from_millis(100), watch_rx.recv()).await {
                Ok(Some(event)) => {
                    for path in event.paths {
                        let event_path = path.canonicalize().unwrap_or(path);
                        for (watch_path, pane_id) in &path_to_pane {
                            if event_path.starts_with(watch_path) {
                                pending_restarts.insert(*pane_id, Instant::now());
                                break;
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => {}
            }

            let now = Instant::now();
            let mut ready_to_restart = Vec::new();

            for (pane_id, last_event_time) in &pending_restarts {
                let settle_time = pane_settle_times
                    .get(pane_id)
                    .copied()
                    .unwrap_or(Duration::from_millis(800));

                if now.duration_since(*last_event_time) >= settle_time {
                    ready_to_restart.push(*pane_id);
                }
            }

            for pane_id in ready_to_restart {
                pending_restarts.remove(&pane_id);
                let _ = tx_watcher.send(PinchEvent::FileChanged(pane_id)).await;
            }
        }
        drop(watcher);
    });

    Ok(handle)
}
