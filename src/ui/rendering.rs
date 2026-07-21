use crate::config::PaneMode;
use crate::process::panes::{LogMode, ProcessState};
use crate::state::AppState;
use crate::ui::layouts::Viewport;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tui_term::widget::PseudoTerminal;

fn draw_combined_logs(state: &AppState, frame: &mut Frame, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let total_logs = state.combined_logs.len();
    let view = Viewport::visible_range(total_logs, inner_height, state.global_view_top);

    let max_name_len = state
        .panes
        .iter()
        .map(|p| p.config.title.chars().count())
        .max()
        .unwrap_or(0);

    let colors = [
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::LightBlue,
        Color::LightRed,
    ];
    let mut list_items = vec![];

    for (id, text_line) in state
        .combined_logs
        .iter()
        .skip(view.start)
        .take(view.end.saturating_sub(view.start))
    {
        let name = state
            .panes
            .iter()
            .find(|p| p.id == *id)
            .map(|p| p.config.title.as_str())
            .unwrap_or("?");
        let tag_color = colors[id % colors.len()];

        let mut text_line = text_line.clone();

        if state.show_combined_prefixes {
            let padded_name = format!("{:>width$}", name, width = max_name_len);
            let prefix_span = Span::styled(
                format!("[{}] ", padded_name),
                Style::default().fg(tag_color).add_modifier(Modifier::BOLD),
            );
            text_line.spans.insert(0, prefix_span);
        }
        list_items.push(text_line);
    }

    let title = if view.is_scrolled {
        format!(" Combined Logs [SCROLLED: +{}] ", total_logs.saturating_sub(view.end))
    } else {
        " Combined Logs ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let paragraph = Paragraph::new(list_items).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_process_pane(state: &AppState, frame: &mut Frame, area: Rect, pane_id: usize) {
    let Some(pane_idx) = state.panes.iter().position(|p| p.id == pane_id) else {
        return;
    };
    let pane = &state.panes[pane_idx];
    let inner_height = area.height.saturating_sub(2) as usize;

    let is_zoomed = state.zoomed_pane == Some(pane.id);
    let border_color = if pane_idx == state.focused_pane {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let (status, status_color) = match pane.state {
        ProcessState::Running => ("RUNNING", Color::LightGreen),
        ProcessState::Stopped => ("STOPPED", Color::LightRed),
        ProcessState::ManuallyStopped => ("MANUAL STOP", Color::DarkGray),
        ProcessState::Restarting => ("RESTARTING", Color::LightYellow),
        ProcessState::PendingAutoRestart => ("PENDING", Color::LightCyan),
    };

    let (btn_toggle_str, btn_toggle_color) = if pane.state == ProcessState::Running {
        (" [■]", Color::Red)
    } else {
        (" [▶]", Color::Green)
    };

    let mut title_spans = vec![
        Span::styled(btn_toggle_str, Style::default().fg(btn_toggle_color)),
        Span::styled(" [↺]", Style::default().fg(Color::Yellow)),
        Span::styled(" [↩]", Style::default().fg(Color::Cyan)),
        Span::styled(" [⤢]", Style::default().fg(Color::LightMagenta)),
        Span::raw(" ["),
        Span::styled(status, Style::default().fg(status_color)),
        Span::raw("]"),
    ];

    if pane.config.mode == PaneMode::Log {
        let total_logs = pane.logs.len();
        let view = Viewport::visible_range(total_logs, inner_height, pane.view_top_index);
        let scroll_status = if view.is_scrolled {
            format!(" [↕ {}]", total_logs.saturating_sub(view.end))
        } else {
            "".to_string()
        };
        let wrap_status = if pane.log_mode == LogMode::Wrap { " [WRAP]" } else { "" };
        let h_scroll_status = if pane.horizontal_scroll > 0 && pane.log_mode == LogMode::Truncate {
            format!(" [↔ {}]", pane.horizontal_scroll)
        } else {
            "".to_string()
        };

        title_spans.push(Span::styled(scroll_status, Style::default().fg(Color::Magenta)));
        title_spans.push(Span::styled(
            format!("{}{}", wrap_status, h_scroll_status),
            Style::default().fg(Color::Magenta),
        ));
    } else {
        let tui_status = if pane.tui_focused {
            " [TUI ⌨ ^X]"
        } else {
            " [TUI ⊘ ↵]"
        };

        let tui_color = if pane.tui_focused {
            Color::LightCyan
        } else {
            Color::DarkGray
        };

        title_spans.push(Span::styled(
            tui_status,
            Style::default().fg(tui_color).add_modifier(Modifier::BOLD),
        ));
    }

    title_spans.push(Span::raw(format!(" {}", pane.config.title)));
    let title_line = Line::from(title_spans);

    let active_borders = if is_zoomed {
        Borders::TOP | Borders::BOTTOM
    } else {
        Borders::ALL
    };
    let block = Block::default()
        .title(title_line)
        .borders(active_borders)
        .border_style(Style::default().fg(border_color));

    match pane.config.mode {
        PaneMode::Log => {
            let total_logs = pane.logs.len();
            let view = Viewport::visible_range(total_logs, inner_height, pane.view_top_index);
            let log_slice: Vec<Line<'static>> = pane
                .logs
                .iter()
                .skip(view.start)
                .take(view.end.saturating_sub(view.start))
                .cloned()
                .collect();

            let mut paragraph = Paragraph::new(log_slice).block(block);
            match pane.log_mode {
                LogMode::Wrap => paragraph = paragraph.wrap(Wrap { trim: false }),
                LogMode::Truncate => paragraph = paragraph.scroll((0, pane.horizontal_scroll as u16)),
            }
            frame.render_widget(paragraph, area);
        }
        PaneMode::Tui => {
            let pseudo_term = PseudoTerminal::new(pane.parser.screen()).block(block);
            frame.render_widget(pseudo_term, area);
        }
    }
}

fn draw_process_grid(state: &AppState, frame: &mut Frame, _grid_area: Rect) {
    for geo in &state.cached_geometries {
        match geo.target {
            crate::ui::layouts::PaneTarget::CombinedLogs => {
                draw_combined_logs(state, frame, geo.area);
            }
            crate::ui::layouts::PaneTarget::Process(pane_id) => {
                draw_process_pane(state, frame, geo.area, pane_id);
            }
        }
    }
}

fn draw_header(state: &AppState, frame: &mut Frame, area: Rect) {
    let header_text = Paragraph::new(format!(" {} ", state.title)).style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header_text, area);
}

fn draw_footer(state: &AppState, frame: &mut Frame, area: Rect) {
    let help_str = if state.show_combined_logs {
        " [^A] Grid View | [p] Prefixes | [↕] Nav | [Enter] Tail "
    } else {
        " [s] Start/Stop | [r] Restart | [^L] Clear | [w] Wrap | [z] Zoom | [^A] All Logs | [^Q] Quit "
    };

    let help_text = Paragraph::new(help_str).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(help_text, area);
}

pub fn draw(state: &AppState, frame: &mut Frame) {
    if state.panes.is_empty() {
        return;
    }

    let [header_area, grid_area, footer_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    draw_header(state, frame, header_area);

    if state.show_combined_logs {
        draw_combined_logs(state, frame, grid_area);
    } else {
        draw_process_grid(state, frame, grid_area);
    }

    draw_footer(state, frame, footer_area);
}
