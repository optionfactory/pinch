use crate::config::PaneMode;
use crate::processes::panes::{LogMode, ProcessState};
use crate::ui::DashboardState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

pub enum UserAction {
    None,
    StopProcess(usize),
    StartProcess(usize),
    RestartProcess(usize),
    ToggleZoom(usize),
    OpenLink(usize),
    NextTab,
}

pub trait RectHitTest {
    fn hit(&self, x: u16, y: u16) -> bool;
}

impl RectHitTest for Rect {
    fn hit(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Encodes a key press as the byte sequence a terminal would send to the
/// application. `application_cursor` mirrors DECCKM as tracked by the pane's
/// vt100 parser (vim/less switch it on) and selects `ESC O x` over `ESC [ x`
/// for cursor keys.
pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Vec<u8> {
    let m = key.modifiers;
    // xterm modifier parameter: 1 + (shift:1 | alt:2 | ctrl:4)
    let modifier_param = 1
        + u8::from(m.contains(KeyModifiers::SHIFT))
        + 2 * u8::from(m.contains(KeyModifiers::ALT))
        + 4 * u8::from(m.contains(KeyModifiers::CONTROL));
    let csi = |final_byte: char| -> Vec<u8> {
        if modifier_param > 1 {
            format!("\x1b[1;{}{}", modifier_param, final_byte).into_bytes()
        } else if application_cursor {
            format!("\x1bO{}", final_byte).into_bytes()
        } else {
            format!("\x1b[{}", final_byte).into_bytes()
        }
    };
    let tilde = |number: u8| -> Vec<u8> {
        if modifier_param > 1 {
            format!("\x1b[{};{}~", number, modifier_param).into_bytes()
        } else {
            format!("\x1b[{}~", number).into_bytes()
        }
    };
    let mut bytes = match key.code {
        KeyCode::Char(c) if m.contains(KeyModifiers::CONTROL) => {
            let lower = c.to_ascii_lowercase();
            match lower {
                'a'..='z' => vec![lower as u8 - b'a' + 1],
                ' ' | '@' | '2' => vec![0],
                '[' | '3' => vec![0x1b],
                '\\' | '4' => vec![0x1c],
                ']' | '5' => vec![0x1d],
                '^' | '6' => vec![0x1e],
                '_' | '7' | '/' => vec![0x1f],
                '?' | '8' => vec![0x7f],
                _ => c.to_string().into_bytes(),
            }
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Backspace => b"\x7f".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Null => vec![0],
        KeyCode::Up => csi('A'),
        KeyCode::Down => csi('B'),
        KeyCode::Right => csi('C'),
        KeyCode::Left => csi('D'),
        KeyCode::Home => csi('H'),
        KeyCode::End => csi('F'),
        KeyCode::Insert => tilde(2),
        KeyCode::Delete => tilde(3),
        KeyCode::PageUp => tilde(5),
        KeyCode::PageDown => tilde(6),
        KeyCode::F(n @ 1..=4) => {
            if modifier_param > 1 {
                format!("\x1b[1;{}{}", modifier_param, (b'P' + n - 1) as char).into_bytes()
            } else {
                format!("\x1bO{}", (b'P' + n - 1) as char).into_bytes()
            }
        }
        KeyCode::F(n @ 5..=12) => tilde([15, 17, 18, 19, 20, 21, 23, 24][(n - 5) as usize]),
        _ => vec![],
    };
    // Alt+<char> is conventionally sent as ESC followed by the character.
    if m.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace) {
        bytes.insert(0, 0x1b);
    }
    bytes
}

pub fn handle_key(state: &mut DashboardState, key: KeyEvent) -> UserAction {
    // Terminals with the kitty keyboard protocol also report Repeat/Release;
    // acting on those would double-fire every shortcut.
    if key.kind != KeyEventKind::Press {
        return UserAction::None;
    }
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

    let focused_id = state.panes[state.focused_pane].id;
    let inner_height = state
        .cached_geometries
        .iter()
        .find(|geo| geo.target == crate::ui::layouts::PaneTarget::Process(focused_id))
        .map(|geo| geo.area.height.saturating_sub(2) as usize)
        .unwrap_or(24);

    let pane = &mut state.panes[state.focused_pane];

    // tui interception
    if pane.config.mode == PaneMode::Tui {
        if pane.tui_focused {
            if code == KeyCode::Char('x') && modifiers.contains(KeyModifiers::CONTROL) {
                pane.tui_focused = false;
                return UserAction::None;
            }
            let application_cursor = pane.parser.screen().application_cursor();
            if let Some(writer) = &mut pane.pty_writer {
                let bytes = encode_key(key, application_cursor);
                if !bytes.is_empty() {
                    let _ = writer.write_all(&bytes);
                    let _ = writer.flush();
                }
            }
            return UserAction::None;
        }

        if code == KeyCode::Enter {
            pane.tui_focused = true;
            return UserAction::None;
        }
    }

    match code {
        KeyCode::Tab => UserAction::NextTab,
        KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
            if pane.config.mode == PaneMode::Log {
                pane.clear_logs();
                pane.add_system_log("LOG BUFFER CLEARED", ratatui::style::Color::DarkGray);
            }
            UserAction::None
        }
        KeyCode::Char('s') => {
            if pane.state == ProcessState::Running {
                UserAction::StopProcess(pane.id)
            } else {
                UserAction::StartProcess(pane.id)
            }
        }
        KeyCode::Char('r') => UserAction::RestartProcess(pane.id),
        KeyCode::Char('w') => {
            pane.toggle_wrap();
            UserAction::None
        }
        KeyCode::Char('z') => UserAction::ToggleZoom(pane.id),
        KeyCode::Enter => {
            pane.view_top_index = None;
            pane.horizontal_scroll = 0;
            UserAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            pane.scroll_up(1, inner_height);
            UserAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            pane.scroll_down(1, inner_height);
            UserAction::None
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if pane.log_mode == LogMode::Truncate {
                pane.horizontal_scroll = pane.horizontal_scroll.saturating_sub(4);
            }
            UserAction::None
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if pane.log_mode == LogMode::Truncate {
                pane.horizontal_scroll = pane.horizontal_scroll.saturating_add(4);
            }
            UserAction::None
        }
        KeyCode::PageUp => {
            pane.scroll_up(10, inner_height);
            UserAction::None
        }
        KeyCode::PageDown => {
            pane.scroll_down(10, inner_height);
            UserAction::None
        }
        _ => UserAction::None,
    }
}

pub fn handle_mouse(state: &mut DashboardState, mouse_event: MouseEvent) -> UserAction {
    if state.show_combined_logs {
        return UserAction::None;
    }
    let mx = mouse_event.column;
    let my = mouse_event.row;

    let Some(geo) = state.cached_geometries.iter().find(|geo| geo.area.hit(mx, my)).cloned() else {
        return UserAction::None;
    };
    if let crate::ui::layouts::PaneTarget::CombinedLogs = geo.target {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => state.scroll_up(2),
            MouseEventKind::ScrollDown => state.scroll_down(2),
            _ => {}
        }
        return UserAction::None;
    }
    let crate::ui::layouts::PaneTarget::Process(proc_id) = geo.target else {
        return UserAction::None;
    };
    let Some(pane_idx) = state.panes.iter().position(|p| p.id == proc_id) else {
        return UserAction::None;
    };

    let inner_height = geo.area.height.saturating_sub(2) as usize;
    let pane = &mut state.panes[pane_idx];

    match mouse_event.kind {
        MouseEventKind::ScrollUp => pane.scroll_up(2, inner_height),
        MouseEventKind::ScrollDown => pane.scroll_down(2, inner_height),
        MouseEventKind::Down(MouseButton::Left) => {
            state.focused_pane = pane_idx;

            if pane.config.mode == PaneMode::Tui && !pane.tui_focused {
                pane.tui_focused = true;
            }

            if my != geo.toggle_area.y {
                return UserAction::None;
            }

            if geo.toggle_area.hit(mx, my) {
                return if pane.state == ProcessState::Running {
                    UserAction::StopProcess(proc_id)
                } else {
                    UserAction::StartProcess(proc_id)
                };
            }
            if geo.restart_area.hit(mx, my) {
                return UserAction::RestartProcess(proc_id);
            }
            if geo.wrap_area.hit(mx, my) {
                pane.toggle_wrap();
                return UserAction::None;
            }
            if geo.zoom_area.hit(mx, my) {
                return UserAction::ToggleZoom(proc_id);
            }
            if geo.link_area.hit(mx, my) {
                return UserAction::OpenLink(proc_id);
            }
        }
        _ => {}
    }
    UserAction::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_mod(code: KeyCode, m: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, m)
    }

    #[test]
    fn plain_chars_are_utf8_encoded() {
        assert_eq!(encode_key(key(KeyCode::Char('a')), false), b"a");
        assert_eq!(encode_key(key(KeyCode::Char('é')), false), "é".as_bytes());
        assert_eq!(encode_key(key(KeyCode::Char('€')), false), "€".as_bytes());
    }

    #[test]
    fn control_chars_map_to_c0_codes() {
        assert_eq!(encode_key(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL), false), vec![0x03]);
        assert_eq!(encode_key(key_mod(KeyCode::Char('Z'), KeyModifiers::CONTROL), false), vec![0x1a]);
        assert_eq!(encode_key(key_mod(KeyCode::Char('['), KeyModifiers::CONTROL), false), vec![0x1b]);
    }

    #[test]
    fn backspace_is_del_and_editing_keys_are_forwarded() {
        assert_eq!(encode_key(key(KeyCode::Backspace), false), vec![0x7f]);
        assert_eq!(encode_key(key(KeyCode::Delete), false), b"\x1b[3~");
        assert_eq!(encode_key(key(KeyCode::Home), false), b"\x1b[H");
        assert_eq!(encode_key(key(KeyCode::End), false), b"\x1b[F");
        assert_eq!(encode_key(key(KeyCode::PageUp), false), b"\x1b[5~");
        assert_eq!(encode_key(key(KeyCode::PageDown), false), b"\x1b[6~");
        assert_eq!(encode_key(key(KeyCode::Insert), false), b"\x1b[2~");
        assert_eq!(encode_key(key(KeyCode::BackTab), false), b"\x1b[Z");
    }

    #[test]
    fn function_keys() {
        assert_eq!(encode_key(key(KeyCode::F(1)), false), b"\x1bOP");
        assert_eq!(encode_key(key(KeyCode::F(4)), false), b"\x1bOS");
        assert_eq!(encode_key(key(KeyCode::F(5)), false), b"\x1b[15~");
        assert_eq!(encode_key(key(KeyCode::F(12)), false), b"\x1b[24~");
    }

    #[test]
    fn cursor_keys_honour_application_mode_and_modifiers() {
        assert_eq!(encode_key(key(KeyCode::Up), false), b"\x1b[A");
        assert_eq!(encode_key(key(KeyCode::Up), true), b"\x1bOA");
        assert_eq!(encode_key(key_mod(KeyCode::Right, KeyModifiers::CONTROL), false), b"\x1b[1;5C");
        assert_eq!(encode_key(key_mod(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::ALT), true), b"\x1b[1;4D");
    }

    #[test]
    fn alt_char_is_escape_prefixed() {
        assert_eq!(encode_key(key_mod(KeyCode::Char('x'), KeyModifiers::ALT), false), b"\x1bx");
    }
}
