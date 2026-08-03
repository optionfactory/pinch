use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout, Write};
use std::panic;
use std::time::{Duration, Instant};

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

fn restore_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, DisableMouseCapture);

    // Prevent pending crossterm mouse events from printing garbage to the shell
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(20) {
        if let Ok(true) = event::poll(Duration::from_millis(5)) {
            let _ = event::read();
        }
    }

    let _ = execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show);
    let _ = disable_raw_mode();
    let _ = stdout.flush();
}

pub struct TerminalGuard {
    pub terminal: TuiTerminal,
}

impl TerminalGuard {
    pub fn init() -> io::Result<Self> {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            restore_terminal();
            original_hook(panic_info);
        }));

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}
