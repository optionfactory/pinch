use crate::config::PinchConfig;
use crate::processes::panes::{ProcessPane, ProcessState};
use crate::processes::ptys::{PtyHandle, spawn_process};
use crate::ui::DashboardState;
use crate::ui::inputs::{UserAction, handle_key, handle_mouse};
use crossterm::event::Event;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum SupervisorEvent {
    Input(Event),
    LogLine(usize, String),
    TerminalBytes(usize, Vec<u8>),
    RestartProcess(usize, bool),
    FileChanged(usize),
    SupervisorTick,
    Error(String),
}

pub struct Supervisor {
    pub state: DashboardState,
    pub ui_tx: mpsc::Sender<SupervisorEvent>,
    pub logs_tx: mpsc::Sender<SupervisorEvent>,
}

impl Supervisor {
    pub fn new(
        config: PinchConfig,
        ui_tx: mpsc::Sender<SupervisorEvent>,
        logs_tx: mpsc::Sender<SupervisorEvent>,
    ) -> Self {
        let panes: Vec<ProcessPane> = config
            .processes
            .iter()
            .enumerate()
            .map(|(id, cfg)| ProcessPane::new(id, config.logs_max_size, cfg.clone()))
            .collect();
        let state = DashboardState {
            name: config.name,
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
        let mut supervisor = Self { state, ui_tx, logs_tx };
        let auto_start_ids: Vec<usize> = supervisor
            .state
            .panes
            .iter()
            .filter(|p| p.config.auto_start)
            .map(|p| p.id)
            .collect();
        for id in auto_start_ids {
            supervisor.start_process(id);
        }
        supervisor
    }

    /// Upper bound on events handled between two redraws: keeps a log firehose
    /// from starving the display while still collapsing bursts into one frame.
    const MAX_EVENTS_PER_FRAME: usize = 256;

    pub async fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
        mut ui_rx: mpsc::Receiver<SupervisorEvent>,
        mut logs_rx: mpsc::Receiver<SupervisorEvent>,
    ) -> Result<(), String> {
        let mut fatal_error = None;
        'frames: loop {
            if let Ok(size) = crossterm::terminal::size() {
                self.sync_pty_sizes(size.0, size.1);
            }
            if let Err(e) = terminal.draw(|f| crate::ui::rendering::draw(&self.state, f)) {
                return Err(format!("Terminal draw failed: {}", e));
            }
            // Block for the first event, then coalesce whatever is already
            // queued so a burst of log lines costs one redraw, not one each.
            // UI events are drained first so input keeps priority over logs.
            let first = tokio::select! {
                biased;
                Some(ev) = ui_rx.recv() => ev,
                Some(ev) = logs_rx.recv() => ev,
                else => break,
            };
            let mut batch = vec![first];
            while batch.len() < Self::MAX_EVENTS_PER_FRAME {
                match ui_rx.try_recv().or_else(|_| logs_rx.try_recv()) {
                    Ok(ev) => batch.push(ev),
                    Err(_) => break,
                }
            }
            for event in batch {
                if let SupervisorEvent::Error(msg) = &event {
                    fatal_error = Some(msg.clone());
                    break 'frames;
                }
                self.handle_event(event);
                if self.state.should_quit {
                    break 'frames;
                }
            }
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
        let DashboardState {
            ref mut panes,
            ref cached_geometries,
            zoomed_pane,
            ..
        } = self.state;
        for geo in cached_geometries {
            if let crate::ui::layouts::PaneTarget::Process(proc_id) = geo.target {
                if let Some(pane) = panes.iter_mut().find(|p| p.id == proc_id) {
                    let is_zoomed = zoomed_pane == Some(pane.id);
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

    pub fn handle_event(&mut self, event: SupervisorEvent) {
        let mut action = UserAction::None;
        match event {
            SupervisorEvent::Input(Event::Mouse(mouse_event)) => {
                action = handle_mouse(&mut self.state, mouse_event);
            }
            SupervisorEvent::Input(Event::Key(key)) => {
                action = handle_key(&mut self.state, key);
            }
            SupervisorEvent::Input(Event::Resize(_, rows)) => {
                self.state.global_area_height = rows.saturating_sub(4) as usize;
                self.state.layout_dirty = true;
            }
            SupervisorEvent::Input(_) => {}
            SupervisorEvent::LogLine(id, line) => {
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
            SupervisorEvent::TerminalBytes(id, bytes) => {
                if let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) {
                    pane.parser.process(&bytes);
                }
            }
            SupervisorEvent::SupervisorTick => {
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
            SupervisorEvent::RestartProcess(id, is_auto) => {
                // This is a continuation scheduled earlier (grace timer or the
                // end of a restart's terminate phase). Only honour it if the pane
                // is still in the state that scheduled it: a manual stop or start
                // in the meantime supersedes it.
                let expected = if is_auto {
                    ProcessState::PendingAutoRestart
                } else {
                    ProcessState::Restarting
                };
                let Some(pane) = self.state.panes.iter().find(|p| p.id == id) else {
                    return;
                };
                if pane.state != expected {
                    return;
                }
                self.launch(id, is_auto);
            }
            SupervisorEvent::FileChanged(id) => self.handle_file_changed(id),
            SupervisorEvent::Error(_) => {}
        }
        match action {
            UserAction::StopProcess(id) => self.stop_process(id),
            UserAction::StartProcess(id) => self.start_process(id),
            UserAction::RestartProcess(id) => self.restart_process(id),
            UserAction::ToggleZoom(id) => self.state.toggle_zoom(id),
            UserAction::OpenLink(id) => {
                if let Some(pane) = self.state.panes.iter().find(|p| p.id == id) {
                    if let Some(link) = pane.config.link.clone() {
                        let url = if link.starts_with("http://") || link.starts_with("https://") {
                            link
                        } else if !link.contains("://") {
                            format!("https://{}", link)
                        } else {
                            // reject non-web URL schemes
                            return;
                        };
                        let _ = open::that(&url);
                    }
                }
            }
            UserAction::NextTab => {
                if !self.state.panes.is_empty() {
                    self.state.focused_pane = (self.state.focused_pane + 1) % self.state.panes.len();
                }
            }
            UserAction::None => {}
        }
    }

    /// User-initiated start (key/click) or boot auto-start. Ignored while the
    /// process is running or a restart is already in flight, so it can never
    /// spawn a second instance next to one that is still being terminated.
    pub fn start_process(&mut self, id: usize) {
        let Some(pane) = self.state.panes.iter().find(|p| p.id == id) else {
            return;
        };
        if matches!(pane.state, ProcessState::Running | ProcessState::Restarting) {
            return;
        }
        self.launch(id, false);
    }

    /// Unconditionally spawns the process into its pane. Callers are
    /// responsible for the state checks.
    fn launch(&mut self, id: usize, is_auto_restart: bool) {
        let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) else {
            return;
        };
        if !is_auto_restart {
            pane.view_top_index = None;
        }
        match spawn_process(id, &pane.config, self.logs_tx.clone()) {
            Ok(PtyHandle { child, writer, master }) => {
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

    pub fn stop_process(&mut self, id: usize) {
        let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) else {
            return;
        };
        match pane.state {
            ProcessState::Running => {
                pane.state = ProcessState::ManuallyStopped;
                pane.terminate();
                pane.add_system_log("STOP SIGNAL SENT (Ctrl+C)", ratatui::style::Color::Red);
            }
            ProcessState::PendingAutoRestart => {
                pane.state = ProcessState::ManuallyStopped;
                pane.add_system_log("PENDING RESTART CANCELLED", ratatui::style::Color::Red);
            }
            ProcessState::Restarting => {
                // The old instance is already being terminated; the pending
                // continuation will see we are no longer `Restarting` and bail.
                pane.state = ProcessState::ManuallyStopped;
                pane.add_system_log("RESTART CANCELLED", ratatui::style::Color::Red);
            }
            ProcessState::Stopped | ProcessState::ManuallyStopped => {}
        }
    }

    pub fn restart_process(&mut self, id: usize) {
        let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) else {
            return;
        };
        match pane.state {
            ProcessState::Running => {
                pane.state = ProcessState::Restarting;
                let terminate_handle = pane.terminate();
                pane.add_system_log("RESTARTING", ratatui::style::Color::Yellow);
                let tx_clone = self.ui_tx.clone();
                tokio::spawn(async move {
                    if let Some(handle) = terminate_handle {
                        let _ = handle.await;
                    }
                    let _ = tx_clone.send(SupervisorEvent::RestartProcess(id, false)).await;
                });
            }
            // Already restarting: the instance about to start will be fresh.
            ProcessState::Restarting => {}
            ProcessState::Stopped | ProcessState::ManuallyStopped | ProcessState::PendingAutoRestart => {
                self.launch(id, false);
            }
        }
    }

    /// A watched path changed. Restarts a running process and starts a crashed
    /// or pending one, but leaves a *manually* stopped process alone: the user
    /// asked for it to be down, and a file save must not override that.
    fn handle_file_changed(&mut self, id: usize) {
        let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) else {
            return;
        };
        let (msg, color, act) = match pane.state {
            ProcessState::Running => ("FILE CHANGED - RESTARTING", ratatui::style::Color::Yellow, true),
            ProcessState::Stopped | ProcessState::PendingAutoRestart => {
                ("FILE CHANGED - STARTING", ratatui::style::Color::Yellow, true)
            }
            ProcessState::Restarting => {
                ("FILE CHANGED - RESTART ALREADY IN PROGRESS", ratatui::style::Color::DarkGray, false)
            }
            ProcessState::ManuallyStopped => {
                ("FILE CHANGED - IGNORED (MANUALLY STOPPED)", ratatui::style::Color::DarkGray, false)
            }
        };
        pane.add_system_log(msg, color);
        if act {
            self.restart_process(id);
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
        // Note: `Restarting` and `ManuallyStopped` never get here, because
        // `terminate()` takes the child out of the pane before the tick loop can
        // observe its exit; those flows are driven by `terminate()`'s handle.
        let Some(pane) = self.state.panes.iter_mut().find(|p| p.id == id) else {
            return;
        };
        if pane.state != ProcessState::Running {
            return;
        }
        pane.state = ProcessState::Stopped;
        pane.child_process = None;
        pane.pty_writer = None;
        pane.pty_master = None;
        pane.last_size = None;
        let (outcome, color) = if success {
            ("EXITED CLEANLY", ratatui::style::Color::Green)
        } else {
            ("CRASHED / FAILED", ratatui::style::Color::Red)
        };
        if !pane.config.auto_restart {
            pane.add_system_log(outcome, color);
            return;
        }
        let grace = pane.config.grace_period;
        pane.state = ProcessState::PendingAutoRestart;
        pane.add_system_log(&format!("{} (Restarting in {}ms)", outcome, grace), color);
        let tx_clone = self.ui_tx.clone();
        tokio::spawn(async move {
            if grace > 0 {
                tokio::time::sleep(Duration::from_millis(grace)).await;
            }
            let _ = tx_clone.send(SupervisorEvent::RestartProcess(id, true)).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PaneMode, ProcessConfig, RunMode};

    fn cfg(name: &str, cmd: &[&str], auto_restart: bool) -> ProcessConfig {
        ProcessConfig {
            name: name.into(),
            title: name.into(),
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
            link: None,
            cwd: None,
            watch: vec![],
            watch_settle_time_ms: 800,
            mode: PaneMode::Log,
            auto_start: false,
            auto_restart,
            grace_period: 0,
            run_mode: RunMode::Exec,
            container: None,
        }
    }

    fn supervisor(processes: Vec<ProcessConfig>) -> (Supervisor, mpsc::Receiver<SupervisorEvent>) {
        let (ui_tx, ui_rx) = mpsc::channel(64);
        let (logs_tx, _logs_rx) = mpsc::channel(64);
        std::mem::forget(_logs_rx); // keep the log channel open for the reader threads
        let config = PinchConfig {
            name: "test".into(),
            processes,
            logs_max_size: None,
            layout: vec![],
        };
        (Supervisor::new(config, ui_tx, logs_tx), ui_rx)
    }

    fn state(s: &Supervisor, id: usize) -> ProcessState {
        s.state.panes[id].state
    }

    fn has_child(s: &Supervisor, id: usize) -> bool {
        s.state.panes[id].child_process.is_some()
    }

    async fn cleanup(mut s: Supervisor) {
        for h in s.shutdown() {
            let _ = h.await;
        }
    }

    #[tokio::test]
    async fn start_is_ignored_while_a_restart_is_in_flight() {
        let (mut s, _rx) = supervisor(vec![cfg("p", &["sleep", "30"], true)]);
        s.start_process(0);
        assert_eq!(state(&s, 0), ProcessState::Running);
        s.restart_process(0);
        assert_eq!(state(&s, 0), ProcessState::Restarting);
        assert!(!has_child(&s, 0), "terminate() takes the child");
        // `s` key / click while restarting used to spawn a duplicate.
        s.start_process(0);
        assert_eq!(state(&s, 0), ProcessState::Restarting);
        assert!(!has_child(&s, 0));
        cleanup(s).await;
    }

    #[tokio::test]
    async fn stop_cancels_an_in_flight_restart() {
        let (mut s, _rx) = supervisor(vec![cfg("p", &["sleep", "30"], true)]);
        s.start_process(0);
        s.restart_process(0);
        s.stop_process(0);
        assert_eq!(state(&s, 0), ProcessState::ManuallyStopped);
        // The continuation the restart scheduled must now be a no-op.
        s.handle_event(SupervisorEvent::RestartProcess(0, false));
        assert_eq!(state(&s, 0), ProcessState::ManuallyStopped);
        assert!(!has_child(&s, 0));
        cleanup(s).await;
    }

    #[tokio::test]
    async fn restart_continuation_starts_the_process_when_still_restarting() {
        let (mut s, _rx) = supervisor(vec![cfg("p", &["sleep", "30"], true)]);
        s.start_process(0);
        s.restart_process(0);
        s.handle_event(SupervisorEvent::RestartProcess(0, false));
        assert_eq!(state(&s, 0), ProcessState::Running);
        assert!(has_child(&s, 0));
        cleanup(s).await;
    }

    #[tokio::test]
    async fn file_change_does_not_resurrect_a_manually_stopped_process() {
        let (mut s, _rx) = supervisor(vec![cfg("p", &["sleep", "30"], true)]);
        s.start_process(0);
        s.stop_process(0);
        assert_eq!(state(&s, 0), ProcessState::ManuallyStopped);
        s.handle_event(SupervisorEvent::FileChanged(0));
        assert_eq!(state(&s, 0), ProcessState::ManuallyStopped);
        assert!(!has_child(&s, 0));
        cleanup(s).await;
    }

    #[tokio::test]
    async fn file_change_starts_a_crashed_process_and_restarts_a_running_one() {
        let (mut s, _rx) = supervisor(vec![cfg("p", &["sleep", "30"], false)]);
        assert_eq!(state(&s, 0), ProcessState::Stopped);
        s.handle_event(SupervisorEvent::FileChanged(0));
        assert_eq!(state(&s, 0), ProcessState::Running);
        s.handle_event(SupervisorEvent::FileChanged(0));
        assert_eq!(state(&s, 0), ProcessState::Restarting);
        cleanup(s).await;
    }

    #[tokio::test]
    async fn auto_restart_continuation_is_ignored_after_a_manual_start_or_stop() {
        let (mut s, _rx) = supervisor(vec![cfg("p", &["sleep", "30"], true)]);
        s.state.panes[0].state = ProcessState::PendingAutoRestart;
        s.stop_process(0);
        assert_eq!(state(&s, 0), ProcessState::ManuallyStopped);
        s.handle_event(SupervisorEvent::RestartProcess(0, true));
        assert_eq!(state(&s, 0), ProcessState::ManuallyStopped);
        assert!(!has_child(&s, 0));

        s.state.panes[0].state = ProcessState::PendingAutoRestart;
        s.start_process(0); // user got impatient
        assert_eq!(state(&s, 0), ProcessState::Running);
        s.handle_event(SupervisorEvent::RestartProcess(0, true));
        assert_eq!(state(&s, 0), ProcessState::Running);
        cleanup(s).await;
    }

    #[tokio::test]
    async fn exit_schedules_auto_restart_only_when_running() {
        let (mut s, mut rx) = supervisor(vec![cfg("p", &["true"], true)]);
        s.start_process(0);
        // wait for `true` to exit, then let the tick observe it
        tokio::time::sleep(Duration::from_millis(300)).await;
        s.handle_event(SupervisorEvent::SupervisorTick);
        assert_eq!(state(&s, 0), ProcessState::PendingAutoRestart);
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        assert!(matches!(ev, SupervisorEvent::RestartProcess(0, true)));
        cleanup(s).await;
    }
}
