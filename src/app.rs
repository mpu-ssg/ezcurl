use ezcurl::domain::client::ports::ClientService;

use crate::{
    action::{Action, Direction},
    editor::{Edit, TextEditor},
    history::{HistoryEntry, HistoryStore},
    request::{HeaderPart, HttpRequest, RequestField},
    response::HttpResponse,
};

const HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Method,
    Url,
    Headers,
    Body,
    Response,
}

pub struct App<C>
where
    C: ClientService,
{
    mode: AppMode,
    focused_panel: Panel,
    response_origin: Panel,

    request: HttpRequest,
    response: Option<HttpResponse>,
    response_error: Option<String>,
    client: C,

    history: Vec<HistoryEntry>,
    history_store: HistoryStore,
    history_storage_error: Option<String>,
    history_open: bool,
    history_selected: usize,
    leader_pending: bool,
    should_quit: bool,
}

impl<C> App<C>
where
    C: ClientService,
{
    pub fn new(request: HttpRequest, client: C, history_store: HistoryStore) -> Self {
        let (mut history, history_storage_error) = match history_store.load() {
            Ok(history) => (history, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        history.truncate(HISTORY_LIMIT);

        Self {
            request,
            response: None,
            response_error: None,
            client,
            mode: AppMode::Normal,
            focused_panel: Panel::Url,
            response_origin: Panel::Url,
            history,
            history_store,
            history_storage_error,
            history_open: false,
            history_selected: 0,
            leader_pending: false,
            should_quit: false,
        }
    }

    pub fn request(&self) -> &HttpRequest {
        &self.request
    }

    pub fn response(&self) -> Option<&HttpResponse> {
        self.response.as_ref()
    }

    pub fn response_error(&self) -> Option<&str> {
        self.response_error.as_deref()
    }

    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    pub fn history_open(&self) -> bool {
        self.history_open
    }

    pub fn history_selected(&self) -> usize {
        self.history_selected
    }

    pub fn history_storage_error(&self) -> Option<&str> {
        self.history_storage_error.as_deref()
    }

    pub fn leader_pending(&self) -> bool {
        self.leader_pending
    }

    pub fn displayed_request(&self) -> &HttpRequest {
        self.selected_history_entry()
            .map(HistoryEntry::request)
            .unwrap_or(&self.request)
    }

    pub fn displayed_response(&self) -> Option<&HttpResponse> {
        self.selected_history_entry()
            .map(HistoryEntry::response)
            .unwrap_or_else(|| self.response())
    }

    pub fn displayed_response_error(&self) -> Option<&str> {
        self.selected_history_entry()
            .map(HistoryEntry::error)
            .unwrap_or_else(|| self.response_error())
    }

    pub fn focused_panel(&self) -> Panel {
        self.focused_panel
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub async fn handle_action(&mut self, action: Action) {
        if self.history_open {
            self.handle_history_action(action);
            return;
        }

        match action {
            Action::Move(direction) => self.move_focus(direction),
            Action::MoveCursor(direction) => self.edit(Edit::Move(direction)),
            Action::MoveCursorToStart => self.edit(Edit::Home),
            Action::MoveCursorToEnd => self.edit(Edit::End),
            Action::NextField => self.next_field(),
            Action::PreviousField => self.previous_field(),
            Action::SendRequest => self.send_request().await,
            Action::NextPanel => self.next_panel(),
            Action::ExitInsert => self.mode = AppMode::Normal,
            Action::EnterInsert | Action::Activate => self.enter_insert_mode(),
            Action::ToggleHistory => self.toggle_history(),
            Action::Leader => self.leader_pending = true,
            Action::CancelLeader => self.leader_pending = false,
            Action::Close => {}
            Action::Quit => self.should_quit = true,
            Action::InsertChar(character) => self.edit(Edit::Insert(character)),
            Action::InsertNewline => self.insert_newline(),
            Action::Backspace => self.edit(Edit::Backspace),
            Action::Delete => self.edit(Edit::Delete),
        }
    }

    pub fn focused_editor(&self) -> Option<&TextEditor> {
        match self.focused_panel {
            Panel::Headers => Some(self.request.header_editor().active_editor()),
            Panel::Method | Panel::Response => None,
            panel => panel
                .request_field()
                .map(|field| self.request.editor(field)),
        }
    }

    fn enter_insert_mode(&mut self) {
        if self.focused_panel != Panel::Response {
            self.mode = AppMode::Insert;
        }
    }

    fn edit(&mut self, edit: Edit) {
        match self.focused_panel {
            Panel::Method => match edit {
                Edit::Move(Direction::Up) => {
                    self.request.set_method(self.request.method().previous())
                }
                Edit::Move(Direction::Down) => {
                    self.request.set_method(self.request.method().next())
                }
                _ => {}
            },
            Panel::Headers => self.request.header_editor_mut().edit(edit),
            Panel::Response => {}
            panel => {
                if let Some(field) = panel.request_field() {
                    self.request.editor_mut(field).edit(edit);
                }
            }
        }
    }

    fn insert_newline(&mut self) {
        match self.focused_panel {
            Panel::Method | Panel::Url => self.mode = AppMode::Normal,
            Panel::Headers => self.request.header_editor_mut().next_field(),
            Panel::Body => self.edit(Edit::Insert('\n')),
            Panel::Response => {}
        }
    }

    fn next_field(&mut self) {
        if self.focused_panel == Panel::Headers {
            self.request.header_editor_mut().next_field();
        } else {
            self.next_panel();
        }
    }

    fn previous_field(&mut self) {
        if self.focused_panel == Panel::Headers {
            self.request.header_editor_mut().previous_field();
            return;
        }

        let previous = match self.focused_panel {
            Panel::Method => Panel::Method,
            Panel::Url => Panel::Method,
            Panel::Headers => Panel::Url,
            Panel::Body => Panel::Headers,
            Panel::Response => Panel::Body,
        };
        self.focus(previous);
    }

    async fn send_request(&mut self) {
        self.mode = AppMode::Normal;
        let request = self.request.clone();
        let result = self.client.execute(request.try_into()?).await;

        let (response, error) = match result {
            Ok(response) => (Some(response), None),
            Err(error) => (None, Some(error.to_string())),
        };

        self.response = response.clone();
        self.response_error = error.clone();
        self.history
            .insert(0, HistoryEntry::new(request, response, error));
        self.history.truncate(HISTORY_LIMIT);
        self.history_selected = 0;
        self.persist_history();
        self.focus(Panel::Response);
    }

    fn toggle_history(&mut self) {
        self.mode = AppMode::Normal;
        self.leader_pending = false;
        self.history_open = !self.history_open;
        self.history_selected = self
            .history_selected
            .min(self.history.len().saturating_sub(1));
    }

    fn handle_history_action(&mut self, action: Action) {
        match action {
            Action::ToggleHistory | Action::Close => {
                self.history_open = false;
                self.leader_pending = false;
            }
            Action::Leader => self.leader_pending = true,
            Action::CancelLeader => self.leader_pending = false,
            Action::Move(Direction::Up) => {
                self.history_selected = self.history_selected.saturating_sub(1)
            }
            Action::Move(Direction::Down) => {
                self.history_selected =
                    (self.history_selected + 1).min(self.history.len().saturating_sub(1))
            }
            Action::Activate => self.restore_history_entry(),
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn restore_history_entry(&mut self) {
        let Some(entry) = self.history.get(self.history_selected).cloned() else {
            return;
        };

        self.request = entry.request().clone();
        self.response = entry.response().cloned();
        self.response_error = entry.error().map(str::to_string);
        self.history_open = false;
        self.focus(Panel::Response);
    }

    fn selected_history_entry(&self) -> Option<&HistoryEntry> {
        self.history_open
            .then(|| self.history.get(self.history_selected))
            .flatten()
    }

    fn persist_history(&mut self) {
        if self.history_storage_error.is_some() {
            return;
        }

        if let Err(error) = self.history_store.save(&self.history) {
            self.history_storage_error = Some(error.to_string());
        }
    }

    fn next_panel(&mut self) {
        let next = match self.focused_panel {
            Panel::Method => Panel::Url,
            Panel::Url => Panel::Headers,
            Panel::Headers => Panel::Body,
            Panel::Body => Panel::Response,
            Panel::Response => Panel::Method,
        };
        self.focus(next);
    }

    fn move_focus(&mut self, direction: Direction) {
        let next = match (self.focused_panel, direction) {
            (Panel::Method, Direction::Right) => Panel::Url,
            (Panel::Method, Direction::Down) => {
                self.request
                    .header_editor_mut()
                    .select_part(HeaderPart::Key);
                Panel::Headers
            }
            (Panel::Url, Direction::Left) => Panel::Method,
            (Panel::Url, Direction::Right) => Panel::Response,
            (Panel::Url, Direction::Down) => {
                self.request
                    .header_editor_mut()
                    .select_part(HeaderPart::Value);
                Panel::Headers
            }
            (Panel::Headers, Direction::Up) => match self.request.header_editor().part() {
                HeaderPart::Key => Panel::Method,
                HeaderPart::Value => Panel::Url,
            },
            (Panel::Headers, Direction::Left) => {
                self.request
                    .header_editor_mut()
                    .select_part(HeaderPart::Key);
                Panel::Headers
            }
            (Panel::Headers, Direction::Right) => {
                if self.request.header_editor().part() == HeaderPart::Key {
                    self.request
                        .header_editor_mut()
                        .select_part(HeaderPart::Value);
                    Panel::Headers
                } else {
                    Panel::Response
                }
            }
            (Panel::Headers, Direction::Down) => Panel::Body,
            (Panel::Body, Direction::Up) => Panel::Headers,
            (Panel::Body, Direction::Right) => Panel::Response,
            (Panel::Response, Direction::Left) => self.response_origin,
            (panel, _) => panel,
        };

        self.focus(next);
    }

    fn focus(&mut self, panel: Panel) {
        if panel == Panel::Response && self.focused_panel != Panel::Response {
            self.response_origin = self.focused_panel;
        }
        self.focused_panel = panel;
    }
}

impl Panel {
    fn request_field(self) -> Option<RequestField> {
        match self {
            Self::Url => Some(RequestField::Url),
            Self::Body => Some(RequestField::Body),
            Self::Method | Self::Headers | Self::Response => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::{App, Panel};
    use crate::{
        action::Direction,
        client::HttpClient,
        editor::Edit,
        history::{HistoryEntry, HistoryStore},
        request::{HeaderPart, HttpMethod, HttpRequest},
    };

    fn app() -> App {
        App::new(
            HttpRequest::new(HttpMethod::Get, "https://example.com".to_string()),
            HttpClient::new(),
            HistoryStore::in_memory(),
        )
    }

    #[test]
    fn method_is_selected_from_the_list() {
        let mut app = app();
        app.move_focus(Direction::Left);

        app.edit(Edit::Move(Direction::Down));

        assert_eq!(app.request().method(), HttpMethod::Post);
    }

    #[test]
    fn vertical_navigation_uses_the_matching_header_column() {
        let mut app = app();

        app.move_focus(Direction::Down);
        assert_eq!(app.focused_panel(), Panel::Headers);

        app.move_focus(Direction::Up);
        assert_eq!(app.focused_panel(), Panel::Url);

        app.move_focus(Direction::Left);
        app.move_focus(Direction::Down);
        app.move_focus(Direction::Up);
        assert_eq!(app.focused_panel(), Panel::Method);
    }

    #[test]
    fn moving_left_from_response_returns_to_the_origin_panel() {
        let mut app = app();

        app.move_focus(Direction::Down);
        app.move_focus(Direction::Right);
        assert_eq!(app.focused_panel(), Panel::Response);

        app.move_focus(Direction::Left);
        assert_eq!(app.focused_panel(), Panel::Headers);
    }

    #[test]
    fn h_and_l_select_header_columns_before_leaving_the_panel() {
        let mut app = app();
        app.move_focus(Direction::Down);
        assert_eq!(app.request().header_editor().part(), HeaderPart::Value);

        app.move_focus(Direction::Left);
        assert_eq!(app.focused_panel(), Panel::Headers);
        assert_eq!(app.request().header_editor().part(), HeaderPart::Key);

        app.move_focus(Direction::Right);
        assert_eq!(app.focused_panel(), Panel::Headers);
        assert_eq!(app.request().header_editor().part(), HeaderPart::Value);

        app.move_focus(Direction::Right);
        assert_eq!(app.focused_panel(), Panel::Response);
    }

    #[test]
    fn restores_a_request_from_history() {
        let mut app = app();
        app.history.push(HistoryEntry::new(
            HttpRequest::new(HttpMethod::Delete, "https://old.test".to_string()),
            None,
            Some("failed".to_string()),
        ));

        app.restore_history_entry();

        assert_eq!(app.request().method(), HttpMethod::Delete);
        assert_eq!(app.request().url(), "https://old.test");
        assert_eq!(app.response_error(), Some("failed"));
    }

    #[test]
    fn history_selection_previews_without_replacing_the_current_request() {
        let mut app = app();
        app.history = vec![
            HistoryEntry::new(
                HttpRequest::new(HttpMethod::Get, "https://first.test".to_string()),
                None,
                None,
            ),
            HistoryEntry::new(
                HttpRequest::new(HttpMethod::Post, "https://second.test".to_string()),
                None,
                Some("failed".to_string()),
            ),
        ];
        app.history_open = true;

        assert_eq!(app.displayed_request().url(), "https://first.test");
        app.handle_history_action(crate::action::Action::Move(Direction::Down));
        assert_eq!(app.displayed_request().url(), "https://second.test");
        assert_eq!(app.displayed_response_error(), Some("failed"));
        assert_eq!(app.request().url(), "https://example.com");
    }

    #[tokio::test]
    async fn sends_a_request_and_records_the_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                )
                .unwrap();
        });
        let history_path =
            std::env::temp_dir().join(format!("ezcurl-app-history-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&history_path);
        let mut app = App::new(
            HttpRequest::new(HttpMethod::Post, format!("http://{address}")),
            HttpClient::new(),
            HistoryStore::new(&history_path),
        );

        app.send_request().await;
        server.join().unwrap();

        assert_eq!(app.response().map(|response| response.status()), Some(201));
        assert_eq!(
            app.response().map(|response| response.body()),
            Some(&b"ok"[..])
        );
        assert_eq!(app.history().len(), 1);
        assert_eq!(app.focused_panel(), Panel::Response);
        assert_eq!(HistoryStore::new(&history_path).load().unwrap().len(), 1);
        std::fs::remove_file(history_path).unwrap();
    }
}
