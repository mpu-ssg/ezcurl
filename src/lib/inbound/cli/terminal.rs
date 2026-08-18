use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

use crate::{
    domain::client::ports::ClientService,
    inbound::cli::{CliError, state::AppState, ui},
};

#[derive(Debug, Clone, Copy)]
pub struct AppControlFlow {
    pub quit: bool,
}

#[derive(Debug)]
pub struct AppTerminal(Terminal<CrosstermBackend<Stdout>>);

impl AppTerminal {
    pub fn enter() -> Result<Self, CliError> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self(terminal))
    }

    pub fn reset() -> Result<(), CliError> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn exit(mut self) -> Result<(), CliError> {
        Self::reset()?;
        self.0.show_cursor()?;

        Ok(())
    }

    pub fn draw<C>(&mut self, state: &mut AppState<C>) -> Result<AppControlFlow, CliError>
    where
        C: ClientService + 'static,
    {
        self.0.draw(|frame| ui::render(frame, state)?);
        Ok(state.control)
    }
}
