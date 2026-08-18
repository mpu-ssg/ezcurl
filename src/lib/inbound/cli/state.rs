use derive_more::Constructor;

use crate::{domain::client::ports::ClientService, inbound::cli::terminal::AppControlFlow};

#[derive(Debug, Constructor)]
pub struct AppState<C>
where
    C: ClientService + 'static,
{
    pub client: C,
    pub control: AppControlFlow,
}
