use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use crate::inbound::cli::{CliError, state::AppState};

pub fn render(frame: &mut Frame, state: &mut AppState<C>) -> Result<(), CliError> {
    todo!()
}
