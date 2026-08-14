use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

use crate::error::EzcurlError;

pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn setup_terminal() -> Result<AppTerminal, EzcurlError> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

pub fn exit_terminal(terminal: &mut AppTerminal) -> Result<(), EzcurlError> {
    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    terminal.show_cursor()?;

    Ok(())
}
