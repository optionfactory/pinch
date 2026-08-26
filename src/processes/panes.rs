use crate::config::ProcessConfig;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::VecDeque;
use std::time::Duration;

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
    pub tui_focused: bool,
    pub parser: vt100::Parser,
    pub last_size: Option<(u16, u16)>,
    pub pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    pub child_process: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    pub pty_writer: Option<Box<dyn std::io::Write + Send>>,
}

const INTERRUPT_GRACE: Duration = Duration::from_secs(3);
// Generous: a database flushing a real datadir on shutdown takes a while.
const TERMINATE_GRACE: Duration = Duration::from_secs(30);

async fn exited_within(child: &mut Box<dyn portable_pty::Child + Send + Sync>, within: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + within;
    loop {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn signal_group(child: &(dyn portable_pty::Child + Send + Sync), signal: libc::c_int) {
    if let Some(pid) = child.process_id().filter(|&p| p > 1) {
        unsafe {
            libc::kill(-(pid as libc::pid_t), signal);
        }
    }
}

impl ProcessPane {
    pub fn new(id: usize, logs_max_size: Option<usize>, config: ProcessConfig) -> Self {
        Self {
            id,
            config,
            logs: VecDeque::new(),
            logs_max_size,
            state: ProcessState::Stopped,
            view_top_index: None,
            horizontal_scroll: 0,
            log_mode: LogMode::Truncate,
            tui_focused: true,
            parser: vt100::Parser::new(24, 80, 0),
            last_size: None,
            pty_master: None,
            child_process: None,
            pty_writer: None,
        }
    }

    pub fn terminate(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        self.pty_master.take();
        // The next spawn opens a fresh 80x24 PTY; forget the old size so
        // `sync_pty_sizes` resizes it on the next frame.
        self.last_size = None;
        if let Some(mut writer) = self.pty_writer.take() {
            let _ = writer.write_all(b"\x03");
            let _ = writer.flush();
        }

        let mut child = self.child_process.take()?;
        Some(tokio::spawn(async move {
            // Some workloads ignore SIGINT (mysqld does), and SIGKILL only reaps the
            // `docker run` client, leaving its container alive.
            if exited_within(&mut child, INTERRUPT_GRACE).await {
                return;
            }
            signal_group(&*child, libc::SIGTERM);
            if exited_within(&mut child, TERMINATE_GRACE).await {
                return;
            }
            let _ = tokio::task::spawn_blocking(move || {
                if child.process_id().is_some_and(|p| p > 1) {
                    signal_group(&*child, libc::SIGKILL);
                } else {
                    let _ = child.kill();
                }
                let _ = child.wait();
            })
            .await;
        }))
    }

    pub fn add_line(&mut self, line: Line<'static>) {
        self.logs.push_back(line);
        let Some(logs_max_size) = self.logs_max_size else {
            return;
        };
        if self.logs.len() <= logs_max_size {
            return;
        }
        self.logs.pop_front();
        // Dropping the oldest line shifts every index by one: move a scrolled
        // viewport along with it so the user keeps looking at the same lines.
        if let Some(top) = self.view_top_index {
            self.view_top_index = Some(top.saturating_sub(1));
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

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.view_top_index = None;
        self.horizontal_scroll = 0;
    }

    pub fn toggle_wrap(&mut self) {
        self.log_mode = match self.log_mode {
            LogMode::Truncate => LogMode::Wrap,
            LogMode::Wrap => LogMode::Truncate,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PaneMode, RunMode};

    fn pane(max: Option<usize>) -> ProcessPane {
        let cfg = ProcessConfig {
            name: "p".into(),
            title: "p".into(),
            cmd: vec!["true".into()],
            link: None,
            cwd: None,
            watch: vec![],
            watch_settle_time_ms: 800,
            mode: PaneMode::Log,
            auto_start: false,
            auto_restart: false,
            grace_period: 0,
            run_mode: RunMode::Exec,
        };
        ProcessPane::new(0, max, cfg)
    }

    fn text(p: &ProcessPane, idx: usize) -> String {
        p.logs[idx].spans.iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn scrolled_view_stays_anchored_when_buffer_is_full() {
        let mut p = pane(Some(3));
        for i in 0..3 {
            p.add_line(Line::from(format!("l{i}")));
        }
        p.view_top_index = Some(1); // looking at l1
        p.add_line(Line::from("l3")); // evicts l0
        assert_eq!(p.logs.len(), 3);
        assert_eq!(p.view_top_index, Some(0));
        assert_eq!(text(&p, 0), "l1");
    }

    #[test]
    fn tail_view_is_unaffected_by_eviction() {
        let mut p = pane(Some(2));
        p.add_line(Line::from("a"));
        p.add_line(Line::from("b"));
        p.add_line(Line::from("c"));
        assert_eq!(p.view_top_index, None);
        assert_eq!(text(&p, 0), "b");
    }

    #[test]
    fn unbounded_buffer_never_evicts() {
        let mut p = pane(None);
        for i in 0..100 {
            p.add_line(Line::from(format!("{i}")));
        }
        assert_eq!(p.logs.len(), 100);
    }
}
