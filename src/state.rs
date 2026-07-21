use crate::config::LayoutBlock;
use crate::process::panes::ProcessPane;
use ratatui::text::Line;
use std::collections::VecDeque;
use crate::ui::layouts::PaneGeometry;
use ratatui::layout::Rect;

pub struct AppState {
    pub title: String,
    pub panes: Vec<ProcessPane>,
    pub focused_pane: usize,
    pub zoomed_pane: Option<usize>,
    pub combined_logs: VecDeque<(usize, Line<'static>)>,
    pub combined_logs_max_size: Option<usize>,
    pub show_combined_logs: bool,
    pub show_combined_prefixes: bool,
    pub global_view_top: Option<usize>,
    pub global_area_height: usize,
    pub should_quit: bool,
    pub layout: Vec<LayoutBlock>,

    pub cached_geometries: Vec<PaneGeometry>,
    pub last_grid_area: Rect,
    pub layout_dirty: bool,    
}

impl AppState {
    pub fn add_global_line(&mut self, id: usize, line: Line<'static>) {
        self.combined_logs.push_back((id, line));
        let Some(max_size) = self.combined_logs_max_size else {
            return;
        };
        if self.combined_logs.len() <= max_size {
            return;
        }
        self.combined_logs.pop_front();
        if let Some(top) = self.global_view_top {
            self.global_view_top = Some(top.saturating_sub(1));
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let current = self
            .global_view_top
            .unwrap_or_else(|| self.combined_logs.len().saturating_sub(self.global_area_height));
        self.global_view_top = Some(current.saturating_sub(amount));
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let Some(top) = self.global_view_top else {
            return;
        };
        let next = top + amount;
        if next >= self.combined_logs.len().saturating_sub(self.global_area_height) {
            self.global_view_top = None;
            return;
        }
        self.global_view_top = Some(next);
    }

    pub fn toggle_zoom(&mut self, id: usize) {
        self.zoomed_pane = if self.zoomed_pane == Some(id) { None } else { Some(id) };
        self.layout_dirty = true;
    }
}
