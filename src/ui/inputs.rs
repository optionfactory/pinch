use crate::config::PaneMode;
use crate::process::panes::{LogMode, ProcessState};
use crate::state::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

pub enum AppAction {
    None,
    StopProcess(usize),
    StartProcess(usize),
    RestartProcess(usize),
    ToggleZoom(usize),
    NextTab,
}

pub fn handle_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    let code = key.code;
    let modifiers = key.modifiers;

    if code == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL) {
        state.should_quit = true;
        return AppAction::None;
    }

    if code == KeyCode::Char('a') && modifiers.contains(KeyModifiers::CONTROL) {
        state.show_combined_logs = !state.show_combined_logs;
        return AppAction::None;
    }

    if state.show_combined_logs {
        match code {
            KeyCode::Char('p') => state.show_combined_prefixes = !state.show_combined_prefixes,
            KeyCode::Up | KeyCode::Char('k') => state.scroll_up(1),
            KeyCode::Down | KeyCode::Char('j') => state.scroll_down(1),
            KeyCode::PageUp => state.scroll_up(10),
            KeyCode::PageDown => state.scroll_down(10),
            KeyCode::Enter => state.global_view_top = None,
            _ => {}
        }
        return AppAction::None;
    }

    if state.panes.is_empty() {
        return AppAction::None;
    }

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

        let geometries = crate::ui::layouts::compute_pane_geometries(
            screen_chunks[1],
            &state.panes,
            state.zoomed_pane,
            &state.layout,
        );
        geometries
            .into_iter()
            .find(|geo| geo.target == crate::ui::layouts::PaneTarget::Process(state.panes[state.focused_pane].id))
            .map(|geo| geo.area.height.saturating_sub(2) as usize)
            .unwrap_or(24)
    } else {
        24
    };

    let mut action = AppAction::None;

    {
        let pane = &mut state.panes[state.focused_pane];

        if pane.config.mode == PaneMode::Tui {
            if pane.tui_focused {
                if code == KeyCode::Char('x') && modifiers.contains(KeyModifiers::CONTROL) {
                    pane.tui_focused = false;
                    return AppAction::None;
                }

                if let Some(writer) = &mut pane.pty_writer {
                    let bytes = match code {
                        KeyCode::Char(c) => {
                            if modifiers.contains(KeyModifiers::CONTROL) {
                                let mapped = c as u8;
                                if (b'a'..=b'z').contains(&mapped) {
                                    vec![mapped - b'a' + 1]
                                } else {
                                    vec![mapped]
                                }
                            } else {
                                vec![c as u8]
                            }
                        }
                        KeyCode::Enter => b"\r".to_vec(),
                        KeyCode::Esc => b"\x1b".to_vec(),
                        KeyCode::Backspace => b"\x08".to_vec(),
                        KeyCode::Up => b"\x1b[A".to_vec(),
                        KeyCode::Down => b"\x1b[B".to_vec(),
                        KeyCode::Right => b"\x1b[C".to_vec(),
                        KeyCode::Left => b"\x1b[D".to_vec(),
                        KeyCode::Tab => b"\t".to_vec(),
                        _ => vec![],
                    };

                    if !bytes.is_empty() {
                        let _ = writer.write_all(&bytes);
                        let _ = writer.flush();
                    }
                }
                return AppAction::None;
            } else {
                if code == KeyCode::Enter {
                    pane.tui_focused = true;
                    return AppAction::None;
                }
            }
        }

        match code {
            KeyCode::Tab => {
                action = AppAction::NextTab;
            }
            KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                if pane.config.mode == PaneMode::Log {
                    pane.clear_logs();
                    pane.add_system_log("LOG BUFFER CLEARED", ratatui::style::Color::DarkGray);
                }
            }
            KeyCode::Char('s') => {
                if pane.state == ProcessState::Running {
                    action = AppAction::StopProcess(pane.id);
                } else {
                    action = AppAction::StartProcess(pane.id);
                }
            }
            KeyCode::Char('r') => action = AppAction::RestartProcess(pane.id),
            KeyCode::Char('w') => pane.toggle_wrap(),
            KeyCode::Char('z') => action = AppAction::ToggleZoom(pane.id),
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

    action
}

pub fn handle_mouse(state: &mut AppState, mouse_event: MouseEvent) -> AppAction {
    if state.show_combined_logs {
        return AppAction::None;
    }

    let mx = mouse_event.column;
    let my = mouse_event.row;

    let (width, height) = match crossterm::terminal::size() {
        Ok(size) => size,
        Err(_) => return AppAction::None,
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

    let geometries =
        crate::ui::layouts::compute_pane_geometries(grid_area, &state.panes, state.zoomed_pane, &state.layout);

    let clicked_geo = geometries.into_iter().find(|geo| {
        mx >= geo.area.x && mx < geo.area.x + geo.area.width && my >= geo.area.y && my < geo.area.y + geo.area.height
    });

    if let Some(geo) = clicked_geo {
        match geo.target {
            crate::ui::layouts::PaneTarget::Process(proc_id) => {
                let Some(pane_idx) = state.panes.iter().position(|p| p.id == proc_id) else {
                    return AppAction::None;
                };

                state.focused_pane = pane_idx;
                let inner_height = geo.area.height.saturating_sub(2) as usize;
                let pane = &mut state.panes[pane_idx];

                if pane.config.mode == PaneMode::Tui && !pane.tui_focused {
                    pane.tui_focused = true;
                }

                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if my == geo.toggle_area.y {
                            if mx >= geo.toggle_area.x && mx < geo.toggle_area.x + geo.toggle_area.width {
                                if pane.state == ProcessState::Running {
                                    return AppAction::StopProcess(proc_id);
                                } else {
                                    return AppAction::StartProcess(proc_id);
                                }
                            } else if mx >= geo.restart_area.x && mx < geo.restart_area.x + geo.restart_area.width {
                                return AppAction::RestartProcess(proc_id);
                            } else if mx >= geo.wrap_area.x && mx < geo.wrap_area.x + geo.wrap_area.width {
                                pane.toggle_wrap();
                            } else if mx >= geo.zoom_area.x && mx < geo.zoom_area.x + geo.zoom_area.width {
                                return AppAction::ToggleZoom(proc_id);
                            }
                        }
                    }
                    MouseEventKind::ScrollUp => pane.scroll_up(2, inner_height),
                    MouseEventKind::ScrollDown => pane.scroll_down(2, inner_height),
                    _ => {}
                }
            }
            crate::ui::layouts::PaneTarget::CombinedLogs => match mouse_event.kind {
                MouseEventKind::ScrollUp => state.scroll_up(2),
                MouseEventKind::ScrollDown => state.scroll_down(2),
                _ => {}
            },
        }
    }

    AppAction::None
}
