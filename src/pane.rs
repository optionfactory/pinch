use std::collections::VecDeque;
use ratatui::text::{Line, Span};
use ratatui::style::{Color, Modifier, Style};
use crate::config::ProcessConfig;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogMode {
    Truncate,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Stopped,
    ManuallyStopped,
    Restarting,
    PendingAutoRestart,
}

pub struct ProcessPane {
    pub id: usize,
    pub config: ProcessConfig,
    pub logs: VecDeque<Line<'static>>,
    pub logs_max_size: Option<usize>,

    pub state: ProcessState,

    pub view_top_index: Option<usize>,
    pub horizontal_scroll: usize,
    pub log_mode: LogMode,
    
    pub child_process: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    pub pty_writer: Option<Box<dyn std::io::Write + Send>>,
}

impl ProcessPane {
    pub fn new(id: usize, logs_max_size: Option<usize>, config: ProcessConfig) -> Self {
        Self {
            id,
            config,
            logs: VecDeque::new(),
            logs_max_size: logs_max_size,
            state: ProcessState::Stopped,
            view_top_index: None,
            horizontal_scroll: 0,
            log_mode: LogMode::Truncate,
            child_process: None,
            pty_writer: None,
        }
    }

    pub fn terminate(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        if let Some(mut writer) = self.pty_writer.take() {
            let _ = writer.write_all(b"\x03"); // Ctrl+C
            let _ = writer.flush();
        }
        
        if let Some(mut child) = self.child_process.take() {
            Some(tokio::task::spawn_blocking(move || {
                let mut exited = false;
                for _ in 0..100 {
                    if let Ok(Some(_)) = child.try_wait() {
                        exited = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }                
                if !exited {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }))
        } else {
            None
        }
    }

    pub fn add_line(&mut self, line: Line<'static>) {
        self.logs.push_back(line);
        if let Some(logs_max_size) = self.logs_max_size {
            if self.logs.len() > logs_max_size {
                self.logs.pop_front();
            }
        }
    }

    pub fn add_system_log(&mut self, msg: &str, color: Color) {
        let span = Span::styled(
            format!(":: {} ::", msg),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        );
        self.add_line(Line::from(vec![span]));
    }    

    pub fn scroll_up(&mut self, amount: usize, inner_height: usize) {
        let current_top = match self.view_top_index {
            None => self.logs.len().saturating_sub(inner_height),
            Some(top) => top,
        };
        self.view_top_index = Some(current_top.saturating_sub(amount));
    }

    pub fn scroll_down(&mut self, amount: usize, inner_height: usize) {
        if let Some(top) = self.view_top_index {
            let next_top = top + amount;
            if next_top >= self.logs.len().saturating_sub(inner_height) {
                self.view_top_index = None;
            } else {
                self.view_top_index = Some(next_top);
            }
        }
    }

    pub fn toggle_wrap(&mut self) {
        self.log_mode = match self.log_mode {
            LogMode::Truncate => LogMode::Wrap,
            LogMode::Wrap => LogMode::Truncate,
        };
    }
}