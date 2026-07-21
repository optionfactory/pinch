use crate::config::PinchConfig;
use crate::events::PinchEvent;
use crate::process::panes::{ProcessPane, ProcessState};
use crate::process::ptys::{PtyProcess, spawn_process};
use crate::state::AppState;
use crate::ui::inputs::{AppAction, handle_key, handle_mouse};
use crossterm::event::Event;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct App {
    pub state: AppState,
    pub ui_tx: mpsc::Sender<PinchEvent>,
    pub logs_tx: mpsc::Sender<PinchEvent>,
}

impl App {
    pub fn new(config: PinchConfig, ui_tx: mpsc::Sender<PinchEvent>, logs_tx: mpsc::Sender<PinchEvent>) -> Self {
        let mut panes: Vec<ProcessPane> = config
            .processes
            .iter()
            .enumerate()
            .map(|(id, cfg)| ProcessPane::new(id, config.logs_max_size, cfg.clone()))
            .collect();

        for pane in panes.iter_mut() {
            if pane.config.auto_start {
                match spawn_process(pane.id, &pane.config, ui_tx.clone(), logs_tx.clone()) {
                    Ok(PtyProcess { child, writer, master }) => {
                        pane.state = ProcessState::Running;
                        pane.child_process = Some(child);
                        pane.pty_writer = Some(writer);
                        pane.pty_master = Some(master);
                        pane.add_system_log("AUTO-STARTED", ratatui::style::Color::Green);
                    }
                    Err(e) => {
                        pane.state = ProcessState::Stopped;
                        pane.add_system_log(&e, ratatui::style::Color::Red);
                    }
                }
            }
        }

        let state = AppState {
            title: config.title,
            panes,
            focused_pane: 0,
            zoomed_pane: None,
            combined_logs: VecDeque::new(),
            combined_logs_max_size: config.logs_max_size,
            show_combined_logs: false,
            show_combined_prefixes: false,
            global_view_top: None,
            global_area_height: 24,
            should_quit: false,
            layout: config.layout,
            cached_geometries: Vec::new(),
            last_grid_area: ratatui::layout::Rect::default(),
            layout_dirty: true,
        };

        Self { state, ui_tx, logs_tx }
    }

    pub async fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
        mut ui_rx: mpsc::Receiver<PinchEvent>,
        mut logs_rx: mpsc::Receiver<PinchEvent>,
    ) -> Result<(), String> {
        let mut fatal_error = None;

        loop {
            if let Ok(size) = crossterm::terminal::size() {
                self.sync_pty_sizes(size.0, size.1);
            }

            if let Err(e) = terminal.draw(|f| crate::ui::rendering::draw(&self.state, f)) {
                return Err(format!("Terminal draw failed: {}", e));
            }

            let event_opt = tokio::select! {
                biased;
                Some(ev) = ui_rx.recv() => Some(ev),
                Some(ev) = logs_rx.recv() => Some(ev),
                else => None,
            };

            if let Some(event) = event_opt {
                if let PinchEvent::Error(msg) = &event {
                    fatal_error = Some(msg.clone());
                    break;
                }
                self.handle_event(event);
                if self.state.should_quit {
                    break;
                }
            } else {
                break;
            }
        }

        println!("Pinch is shutting down... waiting for child processes to exit.");
        for handle in self.shutdown() {
            let _ = handle.await;
        }

        if let Some(err) = fatal_error { Err(err) } else { Ok(()) }
    }

    pub fn sync_pty_sizes(&mut self, term_width: u16, term_height: u16) {
        if self.state.panes.is_empty() || self.state.show_combined_logs {
            return;
        }

        let full_area = ratatui::layout::Rect::new(0, 0, term_width, term_height);
        let screen_chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Min(0),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(full_area);

        let grid_area = screen_chunks[1];

        if self.state.layout_dirty || self.state.last_grid_area != grid_area {
            self.state.cached_geometries = crate::ui::layouts::compute_pane_geometries(
                grid_area,
                &self.state.panes,
                self.state.zoomed_pane,
                &self.state.layout,
            );
            self.state.last_grid_area = grid_area;
            self.state.layout_dirty = false;
        }

        let geometries = self.state.cached_geometries.clone();

        for geo in geometries {
            if let crate::ui::layouts::PaneTarget::Process(proc_id) = geo.target {
                if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == proc_id) {
                    let is_zoomed = self.state.zoomed_pane == Some(pane.id);
                    let rows = geo.area.height.saturating_sub(2);
                    let cols = if is_zoomed {
                        geo.area.width
                    } else {
                        geo.area.width.saturating_sub(2)
                    };
                    if cols > 0 && rows > 0 && pane.last_size != Some((cols, rows)) {
                        pane.last_size = Some((cols, rows));
                        pane.parser.screen_mut().set_size(rows, cols);
                        if let Some(master) = &pane.pty_master {
                            let _ = master.resize(portable_pty::PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: PinchEvent) {
        let mut action = AppAction::None;

        match event {
            PinchEvent::Input(Event::Mouse(mouse_event)) => {
                action = handle_mouse(&mut self.state, mouse_event);
            }
            PinchEvent::Input(Event::Key(key)) => {
                action = handle_key(&mut self.state, key);
            }
            PinchEvent::Input(Event::Resize(_, rows)) => {
                self.state.global_area_height = rows.saturating_sub(4) as usize;
                self.state.layout_dirty = true;
            }
            PinchEvent::Input(_) => {}
            PinchEvent::LogLine(id, line) => {
                use ansi_to_tui::IntoText;

                let parsed_text = line
                    .as_bytes()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::raw(line));

                for text_line in parsed_text.lines {
                    if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
                        pane.add_line(text_line.clone());
                    }
                    self.state.add_global_line(id, text_line);
                }
            }
            PinchEvent::TerminalBytes(id, bytes) => {
                if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
                    pane.parser.process(&bytes);
                }
            }
            PinchEvent::ProcessExit(id, success) => self.handle_process_exit(id, success),
            PinchEvent::SupervisorTick => {
                let mut exited_ids = vec![];
                for pane in &mut self.state.panes {
                    if let Some(child) = &mut pane.child_process {
                        if let Ok(Some(exit_status)) = child.try_wait() {
                            let success = exit_status.success();
                            exited_ids.push((pane.id, success));
                        }
                    }
                }
                for (id, success) in exited_ids {
                    self.handle_process_exit(id, success);
                }
            }
            PinchEvent::RestartProcess(id, is_auto) => {
                if is_auto {
                    if let Some(pane) = self.state.panes.iter().find(|p| p.id == id) {
                        if pane.state != ProcessState::PendingAutoRestart {
                            return;
                        }
                    }
                }
                self.start_process(id, is_auto);
            }
            PinchEvent::FileChanged(id) => {
                if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
                    pane.add_system_log("FILE CHANGED - RESTARTING", ratatui::style::Color::Yellow);
                }
                self.restart_process(id);
            }
            PinchEvent::Error(_) => {}
        }

        match action {
            AppAction::StopProcess(id) => self.stop_process(id),
            AppAction::StartProcess(id) => self.start_process(id, false),
            AppAction::RestartProcess(id) => self.restart_process(id),
            AppAction::ToggleZoom(id) => self.state.toggle_zoom(id),
            AppAction::NextTab => {
                if !self.state.panes.is_empty() {
                    self.state.focused_pane = (self.state.focused_pane + 1) % self.state.panes.len();
                }
            }
            AppAction::None => {}
        }
    }

    pub fn start_process(&mut self, id: usize, is_auto_restart: bool) {
        if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
            if pane.state != ProcessState::Running {
                if !is_auto_restart {
                    pane.view_top_index = None;
                }

                match spawn_process(id, &pane.config, self.ui_tx.clone(), self.logs_tx.clone()) {
                    Ok(PtyProcess { child, writer, master }) => {
                        pane.state = ProcessState::Running;
                        pane.child_process = Some(child);
                        pane.pty_writer = Some(writer);
                        pane.pty_master = Some(master);

                        let msg = if is_auto_restart {
                            "AUTO-RESTARTED"
                        } else {
                            "PROCESS STARTED"
                        };
                        pane.add_system_log(msg, ratatui::style::Color::Green);
                    }
                    Err(e) => {
                        pane.state = ProcessState::Stopped;
                        pane.add_system_log(&e, ratatui::style::Color::Red);
                    }
                }
            }
        }
    }

    pub fn stop_process(&mut self, id: usize) {
        if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
            let old_state = pane.state;

            if old_state == ProcessState::Running {
                pane.state = ProcessState::ManuallyStopped;
                pane.terminate();
                pane.add_system_log("STOP SIGNAL SENT (Ctrl+C)", ratatui::style::Color::Red);
            } else if old_state == ProcessState::PendingAutoRestart {
                pane.state = ProcessState::ManuallyStopped;
                pane.add_system_log("PENDING RESTART CANCELLED", ratatui::style::Color::Red);
            }
        }
    }

    pub fn restart_process(&mut self, id: usize) {
        if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
            if pane.state == ProcessState::Running {
                pane.state = ProcessState::Restarting;
                pane.terminate();
                pane.add_system_log("RESTARTING", ratatui::style::Color::Yellow);
            } else {
                self.start_process(id, false);
            }
        }
    }

    pub fn shutdown(&mut self) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = vec![];
        for pane in self.state.panes.iter_mut() {
            if let Some(handle) = pane.terminate() {
                handles.push(handle);
            }
        }
        handles
    }

    fn handle_process_exit(&mut self, id: usize, success: bool) {
        let mut start_delay = None;
        let mut immediate_restart = false;

        if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
            let old_state = pane.state;

            if old_state == ProcessState::Stopped
                || old_state == ProcessState::PendingAutoRestart
                || old_state == ProcessState::ManuallyStopped
            {
                return;
            }

            pane.state = ProcessState::Stopped;
            pane.child_process = None;
            pane.pty_writer = None;
            pane.pty_master = None;
            pane.last_size = None;

            if old_state == ProcessState::Restarting {
                immediate_restart = true;
            } else if pane.config.auto_restart && old_state == ProcessState::Running {
                pane.state = ProcessState::PendingAutoRestart;
                let grace = pane.config.grace_period;
                let (msg, color) = if success {
                    (
                        format!("EXITED CLEANLY (Restarting in {}ms)", grace),
                        ratatui::style::Color::Green,
                    )
                } else {
                    (
                        format!("CRASHED / FAILED (Restarting in {}ms)", grace),
                        ratatui::style::Color::Red,
                    )
                };
                pane.add_system_log(&msg, color);
                start_delay = Some(grace);
            } else {
                let (msg, color) = if success {
                    ("EXITED CLEANLY".to_string(), ratatui::style::Color::Green)
                } else {
                    ("CRASHED / FAILED".to_string(), ratatui::style::Color::Red)
                };
                pane.add_system_log(&msg, color);
            }
        }

        if immediate_restart {
            let tx_clone = self.ui_tx.clone();
            tokio::spawn(async move {
                let _ = tx_clone.send(PinchEvent::RestartProcess(id, false)).await;
            });
        } else if let Some(delay) = start_delay {
            let tx_clone = self.ui_tx.clone();
            tokio::spawn(async move {
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                let _ = tx_clone.send(PinchEvent::RestartProcess(id, true)).await;
            });
        }
    }
}
