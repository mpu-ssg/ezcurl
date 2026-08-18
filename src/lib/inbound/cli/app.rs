use derive_more::Constructor;

use crate::{
    domain::client::ports::ClientService,
    inbound::cli::{CliError, state::AppState, terminal::AppTerminal},
};

#[derive(Debug, Constructor)]
pub struct App<C>
where
    C: ClientService + 'static,
{
    state: AppState<C>,
}

impl<C> App<C>
where
    C: ClientService + 'static,
{
    pub async fn run(mut self) -> Result<(), CliError> {
        let mut terminal = AppTerminal::enter()?;

        loop {
            if terminal.draw(&mut self.state)?.quit {
                break;
            }
        }

        terminal.exit()
    }
}
