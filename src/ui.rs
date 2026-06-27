use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use crate::{app::App, pane::{LogMode, ProcessState}};

struct Viewport {
    pub start: usize,
    pub end: usize,
    pub is_scrolled: bool,
}

impl Viewport {
    pub fn visible_range(total_items: usize, available_height: usize, top_index: Option<usize>) -> Self {
        match top_index {
            None => {
                let start = total_items.saturating_sub(available_height);
                Self { start, end: total_items, is_scrolled: false }
            }
            Some(top_idx) => {
                let clamped_top = top_idx.min(total_items.saturating_sub(available_height));
                let end = (clamped_top + available_height).min(total_items);
                Self { start: clamped_top, end, is_scrolled: true }
            }
        }
    }
}

fn safe_rect(x: u16, y: u16, width: u16, pane_area: Rect) -> Rect {
    let max_x = pane_area.x + pane_area.width;
    if x >= max_x {
        Rect { x, y, width: 0, height: 0 }
    } else {
        let available = max_x.saturating_sub(x);
        Rect { x, y, width: width.min(available), height: 1 }
    }
}

pub struct PaneGeometry {
    pub id: usize,
    pub area: Rect,
    pub btn_toggle: Rect,
    pub btn_restart: Rect,
    pub btn_wrap: Rect,
    pub btn_zoom: Rect,
}

pub fn compute_pane_geometries(
    grid_area: Rect,
    panes: &[crate::pane::ProcessPane],
    zoomed_pane: Option<usize>,
) -> Vec<PaneGeometry> {
    let mut layouts = vec![];
    if panes.is_empty() { return layouts; }

    let mut render_indices = vec![];

    if let Some(zoom_id) = zoomed_pane {
        if let Some(idx) = panes.iter().position(|p| p.id == zoom_id) {
            render_indices.push((idx, grid_area));
        }
    }

    if zoomed_pane.is_none() {
        let num_panes = panes.len();
        let max_per_col = 4;
        let num_cols = (num_panes + max_per_col - 1) / max_per_col;
        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(100 / num_cols as u16); num_cols])
            .split(grid_area);

        for col_idx in 0..num_cols {
            let start_idx = col_idx * max_per_col;
            let end_idx = std::cmp::min(start_idx + max_per_col, num_panes);
            let items_in_col = end_idx - start_idx;

            let row_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Percentage(100 / items_in_col as u16); items_in_col])
                .split(column_areas[col_idx]);

            for row_idx in 0..items_in_col {
                render_indices.push((start_idx + row_idx, row_areas[row_idx]));
            }
        }
    }

    for (pane_idx, area) in render_indices {
        let pane = &panes[pane_idx];
        let is_zoomed = zoomed_pane == Some(pane.id);
        let base_y = area.y;
        let base_x = if is_zoomed { area.x } else { area.x + 1 };

        layouts.push(PaneGeometry {
            id: pane.id,
            area,
            btn_toggle: safe_rect(base_x, base_y, 4, area),
            btn_restart: safe_rect(base_x + 4, base_y, 4, area),
            btn_wrap: safe_rect(base_x + 8, base_y, 4, area),
            btn_zoom: safe_rect(base_x + 12, base_y, 4, area),
        });
    }

    layouts
}

pub fn draw(f: &mut Frame, app: &App) {
    if app.panes.is_empty() { return; }

    let screen_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let header_area = screen_chunks[0];
    let grid_area = screen_chunks[1];
    let footer_area = screen_chunks[2];

    let header_text = Paragraph::new(format!(" {} ", app.title))
        .style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD));
    f.render_widget(header_text, header_area);

    if app.show_global_logs {
        let total_logs = app.global_logs.len();
        let view = Viewport::visible_range(total_logs, app.global_area_height, app.global_view_top);

        let max_name_len = app.panes.iter().map(|p| p.config.title.chars().count()).max().unwrap_or(0);
        let colors = [Color::Cyan, Color::Green, Color::Yellow, Color::Magenta, Color::LightBlue, Color::LightRed];
        let mut list_items = vec![];

        for (id, text_line) in app.global_logs.iter().skip(view.start).take(view.end.saturating_sub(view.start)) {
            let name = app.panes.iter().find(|p| p.id == *id).map(|p| p.config.title.as_str()).unwrap_or("?");
            let tag_color = colors[id % colors.len()];

            let mut text_line = text_line.clone();
            
            if app.show_global_prefixes {
                let padded_name = format!("{:>width$}", name, width = max_name_len);
                let prefix_span = Span::styled(format!("[{}] ", padded_name), Style::default().fg(tag_color).add_modifier(Modifier::BOLD));
                text_line.spans.insert(0, prefix_span);
            }
            list_items.push(text_line);
        }

        let title = if view.is_scrolled {
            format!(" Combined Logs [SCROLLED: +{}] ", total_logs.saturating_sub(view.end))
        } else {
            " Combined Logs ".to_string()
        };

        let block = Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(Color::Blue));
        let paragraph = Paragraph::new(list_items).block(block).wrap(Wrap { trim: false });
        f.render_widget(paragraph, grid_area);

    } else {
        let geometries = compute_pane_geometries(grid_area, &app.panes, app.zoomed_pane);

        for geo in geometries {
            let pane_idx = app.panes.iter().position(|p| p.id == geo.id).unwrap();
            let pane = &app.panes[pane_idx];
            let inner_height = geo.area.height.saturating_sub(2) as usize;

            let total_logs = pane.logs.len();
            let view = Viewport::visible_range(total_logs, inner_height, pane.view_top_index);
            let log_slice: Vec<Line<'static>> = pane.logs.iter().skip(view.start).take(view.end.saturating_sub(view.start)).cloned().collect();

            let is_zoomed = app.zoomed_pane == Some(pane.id);
            let border_color = if pane_idx == app.focused_pane { Color::Blue } else { Color::DarkGray };
            
            let (status, status_color) = match pane.state {
                ProcessState::Running => ("RUNNING", Color::LightGreen),
                ProcessState::Stopped => ("STOPPED", Color::LightRed),
                ProcessState::ManuallyStopped => ("MANUAL STOP", Color::DarkGray),
                ProcessState::Restarting => ("RESTARTING", Color::LightYellow),
                ProcessState::PendingAutoRestart => ("PENDING", Color::LightCyan),
            };
            
            let (btn_toggle_str, btn_toggle_color) = if pane.state == ProcessState::Running { (" [■]", Color::Red) } else { (" [▶]", Color::Green) };
            let scroll_status = if view.is_scrolled { format!(" [SCROLLED: +{}]", total_logs.saturating_sub(view.end)) } else { "".to_string() };
            let wrap_status = if pane.log_mode == LogMode::Wrap { " [WRAP]" } else { "" };
            let h_scroll_status = if pane.horizontal_scroll > 0 && pane.log_mode == LogMode::Truncate { format!(" [↔ {}]", pane.horizontal_scroll) } else { "".to_string() };

            let title_line = Line::from(vec![
                Span::styled(btn_toggle_str, Style::default().fg(btn_toggle_color)),
                Span::styled(" [↺]", Style::default().fg(Color::Yellow)),
                Span::styled(" [W]", Style::default().fg(Color::Cyan)),
                Span::styled(" [Z]", Style::default().fg(Color::LightMagenta)),
                Span::raw(" ["),
                Span::styled(status, Style::default().fg(status_color)),
                Span::raw("]"),
                Span::styled(scroll_status, Style::default().fg(Color::Magenta)),
                Span::styled(format!("{}{}", wrap_status, h_scroll_status), Style::default().fg(Color::Magenta)),
                Span::raw(format!(" {}", pane.config.title)),
            ]);

            let active_borders = if is_zoomed { Borders::TOP | Borders::BOTTOM } else { Borders::ALL };
            let block = Block::default().title(title_line).borders(active_borders).border_style(Style::default().fg(border_color));

            let mut paragraph = Paragraph::new(log_slice).block(block);
            match pane.log_mode {
                LogMode::Wrap => paragraph = paragraph.wrap(Wrap { trim: false }),
                LogMode::Truncate => paragraph = paragraph.scroll((0, pane.horizontal_scroll as u16)),
            }

            f.render_widget(paragraph, geo.area);
        }
    }

    let help_str = if app.show_global_logs {
        " [a] Grid View | [p] Toggle Prefixes | [↕] Nav | [Enter] Tail "
    } else {
        " [s] Start/Stop | [r] Restart | [w] Wrap | [z] Zoom | [a] All Logs | [^q] Quit "
    };

    let help_text = Paragraph::new(help_str).style(Style::default().bg(Color::DarkGray).fg(Color::White));   
    f.render_widget(help_text, footer_area);
}