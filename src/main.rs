mod action;
mod app;
mod client;
mod editor;
mod error;
mod history;
mod input;
mod request;
mod response;
mod terminal;
mod ui;

use app::App;
use client::HttpClient;
use history::HistoryStore;
use request::{HttpMethod, HttpRequest};
use terminal::setup_terminal;
use ui::draw;

use crate::error::EzcurlError;
use crossterm::event::{self, Event};

async fn run() -> Result<(), EzcurlError> {
    let url = std::env::args().nth(1).unwrap_or_default();

    let mut request = HttpRequest::new(HttpMethod::Get, url);
    request.add_header("User-Agent", "ezcurl/0.1");
    request.add_header("Accept", "text/html");

    let client = HttpClient::new();

    let history_store = HistoryStore::for_current_user()?;
    let mut app = App::new(request, client, history_store);

    let mut terminal = setup_terminal()?;

    while !app.should_quit() {
        terminal.draw(|frame| draw(frame, &app))?;
        let event = event::read()?;

        if let Event::Key(key) = event
            && let Some(action) =
                input::map_key(key, app.mode(), app.focused_panel(), app.leader_pending())
        {
            app.handle_action(action).await;
        }
    }
    let _ = terminal::exit_terminal(&mut terminal);

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Error: {error}");
    }
}
