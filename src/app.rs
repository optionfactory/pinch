use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;
use crate::config::AppConfig;
use crate::pane::{LogMode, ProcessPane, ProcessState};
use crate::process::spawn_process;


#[derive(Debug)]
pub enum AppEvent {
    Input(Event),
    LogLine(usize, String),
    ProcessExit(usize),
    RestartProcess(usize, bool),
    SupervisorTick,
    Error(String),
}

pub struct App {
    pub title: String,
    pub panes: Vec<ProcessPane>,
    pub focused_pane: usize,
    pub zoomed_pane: Option<usize>,
    pub tx: mpsc::Sender<AppEvent>,
    
    pub global_logs: VecDeque<(usize, ratatui::text::Line<'static>)>,
    pub global_logs_max_size: Option<usize>,
    pub show_global_logs: bool,
    pub show_global_prefixes: bool,    
    pub global_view_top: Option<usize>,
    pub global_area_height: usize,
    pub should_quit: bool,    
}

impl App {
    pub fn new(config: AppConfig, tx: mpsc::Sender<AppEvent>) -> Self {
        let mut panes: Vec<ProcessPane> = config
            .processes
            .into_iter()
            .enumerate()
            .map(|(id, cfg)| ProcessPane::new(id, config.logs_max_size, cfg))
            .collect();

        for pane in panes.iter_mut() {
            if pane.config.auto_start {
                pane.state = ProcessState::Running;
                let (child, writer) = spawn_process(pane.id, &pane.config, tx.clone());
                pane.child_process = Some(child);
                pane.pty_writer = Some(writer);
                pane.add_system_log("AUTO-STARTED", ratatui::style::Color::Green);
            }
        }

        Self {
            title: config.title,
            panes,
            focused_pane: 0,
            zoomed_pane: None,
            tx,
            global_logs: VecDeque::new(),
            global_logs_max_size: config.logs_max_size,
            show_global_logs: false,
            show_global_prefixes: false,
            global_view_top: None,
            global_area_height: 24,
            should_quit: false,            
        }
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Input(Event::Mouse(mouse_event)) => self.handle_mouse(mouse_event),
            AppEvent::Input(Event::Key(key)) => self.handle_key(key),            
            AppEvent::Input(Event::Resize(_, rows)) => {
                self.global_area_height = rows.saturating_sub(4) as usize;
            }            
            AppEvent::Input(_) => {}
            AppEvent::LogLine(id, line) => {
                use ansi_to_tui::IntoText;

                let parsed_text = line
                    .as_bytes()
                    .into_text()
                    .unwrap_or_else(|_| ratatui::text::Text::raw(line));

                for text_line in parsed_text.lines {
                    if let Some(pane) = self.panes.iter_mut().find(|p| p.id == id) {
                        pane.add_line(text_line.clone());
                    }
                    self.add_global_line(id, text_line);
                }
            }
            AppEvent::ProcessExit(id) => self.handle_process_exit(id),
            AppEvent::RestartProcess(id, is_auto) => {
                if is_auto {
                    if let Some(pane) = self.panes.iter().find(|p| p.id == id) {
                        if pane.state != ProcessState::PendingAutoRestart {
                            return; 
                        }
                    }
                }
                self.start_process(id, is_auto);                
            },
            AppEvent::SupervisorTick => {
                let mut exited_ids = vec![];
                for pane in &mut self.panes {
                    if let Some(child) = &mut pane.child_process {
                        if let Ok(Some(_exit_status)) = child.try_wait() {
                            exited_ids.push(pane.id);
                        }
                    }
                }
                for id in exited_ids {
                    self.handle_process_exit(id);
                }
            },            
            AppEvent::Error(_) => {
                //nothing to do, handled by main
            },
        }
    }
    
    fn add_global_line(&mut self, id: usize, line: ratatui::text::Line<'static>) {
        self.global_logs.push_back((id, line));
        if let Some(global_logs_max_size) = self.global_logs_max_size {
            if self.global_logs.len() > global_logs_max_size {
                self.global_logs.pop_front();
                if let Some(top) = self.global_view_top {
                    self.global_view_top = Some(top.saturating_sub(1));
                }
            }
        }
    }

    fn global_scroll_up(&mut self, amount: usize) {
        let current = self.global_view_top.unwrap_or_else(|| self.global_logs.len().saturating_sub(self.global_area_height));
        self.global_view_top = Some(current.saturating_sub(amount));
    }

    fn global_scroll_down(&mut self, amount: usize) {
        if let Some(top) = self.global_view_top {
            let next = top + amount;
            if next >= self.global_logs.len().saturating_sub(self.global_area_height) {
                self.global_view_top = None;
            } else {
                self.global_view_top = Some(next);
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse_event: crossterm::event::MouseEvent) {
        if self.show_global_logs { return; }

        let mx = mouse_event.column;
        let my = mouse_event.row;

        let (width, height) = match crossterm::terminal::size() {
            Ok(size) => size,
            Err(_) => return,
        };

        let full_area = ratatui::layout::Rect::new(0, 0, width, height);
        let screen_chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Min(0),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(full_area);
        let grid_area = screen_chunks[1];

        let geometries = crate::ui::compute_pane_geometries(grid_area, &self.panes, self.zoomed_pane);
        
        let clicked_geo = geometries.into_iter().find(|geo| {
            mx >= geo.area.x && mx < geo.area.x + geo.area.width &&
            my >= geo.area.y && my < geo.area.y + geo.area.height
        });

        if let Some(geo) = clicked_geo {
            let pane_idx = self.panes.iter().position(|p| p.id == geo.id).unwrap();
            self.focused_pane = pane_idx;
            
            let inner_height = geo.area.height.saturating_sub(2) as usize;
            let pane = &mut self.panes[pane_idx];

            match mouse_event.kind {
                crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    if my == geo.btn_toggle.y {
                        if mx >= geo.btn_toggle.x && mx < geo.btn_toggle.x + geo.btn_toggle.width {
                            if pane.state == ProcessState::Running { self.stop_process(geo.id); } else { self.start_process(geo.id, false); }
                        } else if mx >= geo.btn_restart.x && mx < geo.btn_restart.x + geo.btn_restart.width {
                            self.restart_process(geo.id);
                        } else if mx >= geo.btn_wrap.x && mx < geo.btn_wrap.x + geo.btn_wrap.width {
                            pane.toggle_wrap();
                        } else if mx >= geo.btn_zoom.x && mx < geo.btn_zoom.x + geo.btn_zoom.width {
                            self.toggle_zoom(geo.id);
                        }
                    }
                }
                crossterm::event::MouseEventKind::ScrollUp => pane.scroll_up(2, inner_height),
                crossterm::event::MouseEventKind::ScrollDown => pane.scroll_down(2, inner_height),
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let code = key.code;
        let modifiers = key.modifiers;        

        if code == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if code == KeyCode::Char('a') {
            self.show_global_logs = !self.show_global_logs;
            return;
        }

        if self.show_global_logs {
            match code {
                KeyCode::Char('p') => self.show_global_prefixes = !self.show_global_prefixes,
                KeyCode::Up | KeyCode::Char('k') => self.global_scroll_up(1),
                KeyCode::Down | KeyCode::Char('j') => self.global_scroll_down(1),
                KeyCode::PageUp => self.global_scroll_up(10),
                KeyCode::PageDown => self.global_scroll_down(10),
                KeyCode::Enter => self.global_view_top = None,
                _ => {}
            }
            return;
        }

        if self.panes.is_empty() { return; }

        let inner_height = if let Ok((width, height)) = crossterm::terminal::size() {
            let full_area = ratatui::layout::Rect::new(0, 0, width, height);
            let screen_chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Min(0),
                    ratatui::layout::Constraint::Length(1),
                ])
                .split(full_area);
            
            let geometries = crate::ui::compute_pane_geometries(screen_chunks[1], &self.panes, self.zoomed_pane);
            geometries.into_iter()
                .find(|geo| geo.id == self.panes[self.focused_pane].id)
                .map(|geo| geo.area.height.saturating_sub(2) as usize)
                .unwrap_or(24)
        } else {
            24
        };

        let pane = &mut self.panes[self.focused_pane];

        match code {
            KeyCode::Tab => self.focused_pane = (self.focused_pane + 1) % self.panes.len(),
            KeyCode::Char('s') => {
                if pane.state == ProcessState::Running {
                    self.stop_process(self.focused_pane);
                } else {
                    self.start_process(self.focused_pane, false);
                }
            }
            KeyCode::Char('r') => self.restart_process(self.focused_pane),
            KeyCode::Char('w') => pane.toggle_wrap(),
            KeyCode::Char('z') => self.toggle_zoom(self.focused_pane),
            KeyCode::Enter => {
                pane.view_top_index = None;
                pane.horizontal_scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => pane.scroll_up(1, inner_height),
            KeyCode::Down | KeyCode::Char('j') => pane.scroll_down(1, inner_height),
            KeyCode::Left | KeyCode::Char('h') => {
                if pane.log_mode == LogMode::Truncate {
                    pane.horizontal_scroll = pane.horizontal_scroll.saturating_sub(4);
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if pane.log_mode == LogMode::Truncate {
                    pane.horizontal_scroll = pane.horizontal_scroll.saturating_add(4);
                }
            }
            KeyCode::PageUp => pane.scroll_up(10, inner_height),
            KeyCode::PageDown => pane.scroll_down(10, inner_height),
            _ => {}
        }
    }

    pub fn start_process(&mut self, id: usize, is_auto_restart: bool) {
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == id) {
            if pane.state != ProcessState::Running {
                pane.state = ProcessState::Running;
                if !is_auto_restart { pane.view_top_index = None; }
                
                let (child, writer) = spawn_process(id, &pane.config, self.tx.clone());
                pane.child_process = Some(child);
                pane.pty_writer = Some(writer);
                
                let msg = if is_auto_restart { "AUTO-RESTARTED" } else { "PROCESS STARTED" };
                pane.add_system_log(msg, ratatui::style::Color::Green);
            }
        }
    }

    pub fn stop_process(&mut self, id: usize) {
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == id) {
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
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == id) {
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
        for pane in self.panes.iter_mut() {
            if let Some(handle) = pane.terminate() {
                handles.push(handle);
            }
        }
        handles
    }

    fn toggle_zoom(&mut self, id: usize) {
        if self.zoomed_pane == Some(id) { self.zoomed_pane = None; } else { self.zoomed_pane = Some(id); }
    }

    fn handle_process_exit(&mut self, id: usize) {
        let mut start_delay = None;
        let mut immediate_restart = false;

        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == id) {
            let old_state = pane.state;            
            
            if old_state == ProcessState::Stopped || old_state == ProcessState::PendingAutoRestart || old_state == ProcessState::ManuallyStopped {
                return;
            }

            pane.state = ProcessState::Stopped;
            pane.child_process = None;
            pane.pty_writer = None;

            if old_state == ProcessState::Restarting {
                immediate_restart = true;
            } else if pane.config.auto_restart && old_state == ProcessState::Running {
                pane.state = ProcessState::PendingAutoRestart;
                let grace = pane.config.grace_period;
                pane.add_system_log(&format!("EXITED (Restarting in {}ms)", grace), ratatui::style::Color::DarkGray);
                start_delay = Some(grace);
            } else {
                pane.add_system_log("EXITED", ratatui::style::Color::DarkGray);
            }
        }

        if immediate_restart {
            let tx_clone = self.tx.clone();
            tokio::spawn(async move { let _ = tx_clone.send(AppEvent::RestartProcess(id, false)).await; });
        } else if let Some(delay) = start_delay {
            let tx_clone = self.tx.clone();
            tokio::spawn(async move {
                if delay > 0 { tokio::time::sleep(Duration::from_millis(delay)).await; }
                let _ = tx_clone.send(AppEvent::RestartProcess(id, true)).await;
            });
        }
    }
}