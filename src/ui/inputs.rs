use crate::config::PaneMode;
use crate::dashboard::DashboardState;
use crate::process::panes::{LogMode, ProcessState};
use crate::ui::layouts::RectHitTest;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

pub enum UserAction {
    None,
    StopProcess(usize),
    StartProcess(usize),
    RestartProcess(usize),
    ToggleZoom(usize),
    OpenLink(usize),
    NextTab,
}

pub fn handle_key(state: &mut DashboardState, key: KeyEvent) -> UserAction {
    let code = key.code;
    let modifiers = key.modifiers;

    if code == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL) {
        state.should_quit = true;
        return UserAction::None;
    }

    if code == KeyCode::Char('a') && modifiers.contains(KeyModifiers::CONTROL) {
        state.show_combined_logs = !state.show_combined_logs;
        return UserAction::None;
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
        return UserAction::None;
    }

    if state.panes.is_empty() {
        return UserAction::None;
    }

    let inner_height = state
        .cached_geometries
        .iter()
        .find(|geo| geo.target == crate::ui::layouts::PaneTarget::Process(state.panes[state.focused_pane].id))
        .map(|geo| geo.area.height.saturating_sub(2) as usize)
        .unwrap_or(24);

    let mut action = UserAction::None;
    {
        let pane = &mut state.panes[state.focused_pane];
        if pane.config.mode == PaneMode::Tui {
            if pane.tui_focused {
                if code == KeyCode::Char('x') && modifiers.contains(KeyModifiers::CONTROL) {
                    pane.tui_focused = false;
                    return UserAction::None;
                }
                if let Some(writer) = &mut pane.pty_writer {
                    let bytes = match code {
                        KeyCode::Char(c) => {
                            if modifiers.contains(KeyModifiers::CONTROL) {
                                let mapped = c.to_ascii_lowercase() as u8;
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
                return UserAction::None;
            } else {
                if code == KeyCode::Enter {
                    pane.tui_focused = true;
                    return UserAction::None;
                }
            }
        }
        match code {
            KeyCode::Tab => {
                action = UserAction::NextTab;
            }
            KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                if pane.config.mode == PaneMode::Log {
                    pane.clear_logs();
                    pane.add_system_log("LOG BUFFER CLEARED", ratatui::style::Color::DarkGray);
                }
            }
            KeyCode::Char('s') => {
                if pane.state == ProcessState::Running {
                    action = UserAction::StopProcess(pane.id);
                } else {
                    action = UserAction::StartProcess(pane.id);
                }
            }
            KeyCode::Char('r') => action = UserAction::RestartProcess(pane.id),
            KeyCode::Char('w') => pane.toggle_wrap(),
            KeyCode::Char('z') => action = UserAction::ToggleZoom(pane.id),
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

pub fn handle_mouse(state: &mut DashboardState, mouse_event: MouseEvent) -> UserAction {
    if state.show_combined_logs {
        return UserAction::None;
    }
    let mx = mouse_event.column;
    let my = mouse_event.row;

    let clicked_geo = state.cached_geometries.iter().find(|geo| geo.area.hit(mx, my)).cloned();

    if let Some(geo) = clicked_geo {
        match geo.target {
            crate::ui::layouts::PaneTarget::Process(proc_id) => {
                let Some(pane_idx) = state.panes.iter().position(|p| p.id == proc_id) else {
                    return UserAction::None;
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
                            if geo.toggle_area.hit(mx, my) {
                                if pane.state == ProcessState::Running {
                                    return UserAction::StopProcess(proc_id);
                                } else {
                                    return UserAction::StartProcess(proc_id);
                                }
                            } else if geo.restart_area.hit(mx, my) {
                                return UserAction::RestartProcess(proc_id);
                            } else if geo.wrap_area.hit(mx, my) {
                                pane.toggle_wrap();
                            } else if geo.zoom_area.hit(mx, my) {
                                return UserAction::ToggleZoom(proc_id);
                            } else if geo.link_area.hit(mx, my) {
                                return UserAction::OpenLink(proc_id);
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
    UserAction::None
}
