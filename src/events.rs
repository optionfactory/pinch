use crossterm::event::Event;

#[derive(Debug)]
pub enum SupervisorEvent {
    Input(Event),
    LogLine(usize, String),
    TerminalBytes(usize, Vec<u8>),
    RestartProcess(usize, bool),
    FileChanged(usize),
    SupervisorTick,
    Error(String),
}
