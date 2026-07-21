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

const COLOR_BORDER_ACTIVE: Color = Color::Rgb(122, 162, 247); // Electric Blue
const COLOR_BORDER_INACTIVE: Color = Color::Rgb(86, 95, 137); // Dim Gray

const COLOR_HEADER_BG: Color = Color::Rgb(45, 63, 118); // Deep Navy
const COLOR_FOOTER_BG: Color = Color::Rgb(30, 32, 48); // Deep Slate
const COLOR_MUTED_TEXT: Color = Color::Rgb(169, 177, 214); // Soft Blue-Gray

const COLOR_PANE_TITLE: Color = Color::Rgb(137, 220, 255); // Azure/Cyan tint
const COLOR_PANE_ACCENT: Color = Color::Rgb(187, 154, 247); // Neon Purple (Used for zoom/scroll status)

const COLOR_RUNNING: Color = Color::Rgb(158, 206, 106); // Bright Green
const COLOR_STOPPED: Color = Color::Rgb(247, 118, 142); // Vibrant Pink/Red
const COLOR_RESTARTING: Color = Color::Rgb(255, 158, 100); // Neon Orange
const COLOR_PENDING: Color = Color::Rgb(125, 207, 255); // Bright Cyan

const COLOR_LOG_PREFIX_PINK: Color = Color::Rgb(255, 150, 236); // Hot Pink

const LOG_COLORS: [Color; 6] = [
    COLOR_PENDING,
    COLOR_RUNNING,
    COLOR_RESTARTING,
    COLOR_PANE_ACCENT,
    COLOR_LOG_PREFIX_PINK,
    COLOR_BORDER_ACTIVE,
];

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
        let tag_color = LOG_COLORS[id % LOG_COLORS.len()];

        let mut spans = Vec::with_capacity(text_line.spans.len() + 1);

        if state.show_combined_prefixes {
            let padded_name = format!("{:>width$}", name, width = max_name_len);
            let prefix_span = Span::styled(
                format!("[{}] ", padded_name),
                Style::default().fg(tag_color).add_modifier(Modifier::BOLD),
            );
            spans.push(prefix_span);
        }

        spans.extend(text_line.spans.iter().map(|s| {
            Span::styled(s.content.as_ref(), s.style)
        }));

        list_items.push(Line::from(spans));
    }

    let title = if view.is_scrolled {
        format!(" Combined Logs [SCROLLED: +{}] ", total_logs.saturating_sub(view.end))
    } else {
        " Combined Logs ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_ACTIVE));
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
        COLOR_BORDER_ACTIVE
    } else {
        COLOR_BORDER_INACTIVE
    };

    let (status, status_color) = match pane.state {
        ProcessState::Running => ("RUNNING", COLOR_RUNNING),
        ProcessState::Stopped => ("STOPPED", COLOR_STOPPED),
        ProcessState::ManuallyStopped => ("MANUAL STOP", COLOR_MUTED_TEXT),
        ProcessState::Restarting => ("RESTARTING", COLOR_RESTARTING),
        ProcessState::PendingAutoRestart => ("PENDING", COLOR_PENDING),
    };

    let (btn_toggle_str, btn_toggle_color) = if pane.state == ProcessState::Running {
        (" [■]", COLOR_STOPPED)
    } else {
        (" [▶]", COLOR_RUNNING)
    };

    let mut title_spans = vec![
        Span::styled(btn_toggle_str, Style::default().fg(btn_toggle_color)),
        Span::styled(" [↺]", Style::default().fg(COLOR_RESTARTING)),
        Span::styled(" [↩]", Style::default().fg(COLOR_PENDING)),
        Span::styled(" [⤢]", Style::default().fg(COLOR_PANE_ACCENT)),
        Span::styled(" [", Style::default().fg(COLOR_BORDER_INACTIVE)),
        Span::styled(status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::styled("]", Style::default().fg(COLOR_BORDER_INACTIVE)),
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

        title_spans.push(Span::styled(scroll_status, Style::default().fg(COLOR_PANE_ACCENT)));
        title_spans.push(Span::styled(
            format!("{}{}", wrap_status, h_scroll_status),
            Style::default().fg(COLOR_PANE_ACCENT),
        ));
    } else {
        let tui_status = if pane.tui_focused {
            " [TUI ⌨ ^X]"
        } else {
            " [TUI ⊘ ↵]"
        };

        let tui_color = if pane.tui_focused {
            COLOR_PENDING
        } else {
            COLOR_MUTED_TEXT
        };

        title_spans.push(Span::styled(
            tui_status,
            Style::default().fg(tui_color).add_modifier(Modifier::BOLD),
        ));
    }

    title_spans.push(Span::styled(
        format!(" {}", pane.config.title),
        Style::default().fg(COLOR_PANE_TITLE).add_modifier(Modifier::BOLD)
    ));
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
            let log_slice: Vec<Line> = pane
                .logs
                .iter()
                .skip(view.start)
                .take(view.end.saturating_sub(view.start))
                .map(|line| {
                    let borrowed_spans: Vec<Span> = line.spans.iter().map(|s| {
                        Span::styled(s.content.as_ref(), s.style)
                    }).collect();
                    Line::from(borrowed_spans)
                })
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
            .bg(COLOR_HEADER_BG)
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

    let help_text = Paragraph::new(help_str).style(
        Style::default()
            .bg(COLOR_FOOTER_BG)
            .fg(COLOR_MUTED_TEXT)
    );
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