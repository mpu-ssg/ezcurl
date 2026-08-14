use crate::{
    action::Direction,
    editor::{Edit, TextEditor},
    error::EzcurlError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub const ALL: [Self; 7] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
        Self::Head,
        Self::Options,
    ];

    pub fn as_reqwest_method(self) -> reqwest::Method {
        match self {
            Self::Get => reqwest::Method::GET,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
            Self::Patch => reqwest::Method::PATCH,
            Self::Delete => reqwest::Method::DELETE,
            Self::Head => reqwest::Method::HEAD,
            Self::Options => reqwest::Method::OPTIONS,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|method| *method == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|method| *method == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RequestField {
    Url,
    Body,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    method: HttpMethod,
    url: TextEditor,
    headers: HeaderEditor,
    body: TextEditor,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: String) -> Self {
        Self {
            method,
            url: TextEditor::new(url),
            headers: HeaderEditor::default(),
            body: TextEditor::default(),
        }
    }

    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.add(key, value);
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = TextEditor::new(String::from_utf8_lossy(&body).into_owned());
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn set_method(&mut self, method: HttpMethod) {
        self.method = method;
    }

    pub fn url(&self) -> &str {
        self.url.text()
    }

    pub fn header_values(&self) -> Result<Vec<(String, String)>, EzcurlError> {
        self.headers.values()
    }

    pub fn body(&self) -> Option<&[u8]> {
        (!self.body.text().is_empty()).then(|| self.body.text().as_bytes())
    }

    pub fn editor(&self, field: RequestField) -> &TextEditor {
        match field {
            RequestField::Url => &self.url,
            RequestField::Body => &self.body,
        }
    }

    pub fn editor_mut(&mut self, field: RequestField) -> &mut TextEditor {
        match field {
            RequestField::Url => &mut self.url,
            RequestField::Body => &mut self.body,
        }
    }

    pub fn header_editor(&self) -> &HeaderEditor {
        &self.headers
    }

    pub fn header_editor_mut(&mut self) -> &mut HeaderEditor {
        &mut self.headers
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderPart {
    Key,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderState {
    Included,
    Pending,
    Invalid,
}

#[derive(Debug, Default, Clone)]
pub struct HeaderRow {
    key: TextEditor,
    value: TextEditor,
}

impl HeaderRow {
    fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: TextEditor::new(key),
            value: TextEditor::new(value),
        }
    }

    pub fn key(&self) -> &str {
        self.key.text()
    }

    pub fn value(&self) -> &str {
        self.value.text()
    }

    pub fn state(&self) -> HeaderState {
        match (self.key().is_empty(), self.value().is_empty()) {
            (false, _) => HeaderState::Included,
            (true, false) => HeaderState::Invalid,
            (true, true) => HeaderState::Pending,
        }
    }

    fn is_empty(&self) -> bool {
        self.key().is_empty() && self.value().is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct HeaderEditor {
    rows: Vec<HeaderRow>,
    selected: usize,
    part: HeaderPart,
}

impl Default for HeaderEditor {
    fn default() -> Self {
        Self {
            rows: vec![HeaderRow::default()],
            selected: 0,
            part: HeaderPart::Key,
        }
    }
}

impl HeaderEditor {
    pub fn rows(&self) -> &[HeaderRow] {
        &self.rows
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn part(&self) -> HeaderPart {
        self.part
    }

    pub fn select_part(&mut self, part: HeaderPart) {
        self.part = part;
    }

    pub fn active_editor(&self) -> &TextEditor {
        match self.part {
            HeaderPart::Key => &self.rows[self.selected].key,
            HeaderPart::Value => &self.rows[self.selected].value,
        }
    }

    pub fn edit(&mut self, edit: Edit) {
        match edit {
            Edit::Move(Direction::Left)
                if self.part == HeaderPart::Value && self.active_editor().is_at_start() =>
            {
                self.part = HeaderPart::Key;
                self.rows[self.selected].key.edit(Edit::End);
            }
            Edit::Move(Direction::Right)
                if self.part == HeaderPart::Key && self.active_editor().is_at_end() =>
            {
                self.part = HeaderPart::Value;
                self.rows[self.selected].value.edit(Edit::Home);
            }
            Edit::Move(Direction::Up) => self.selected = self.selected.saturating_sub(1),
            Edit::Move(Direction::Down) => {
                self.selected = (self.selected + 1).min(self.rows.len() - 1)
            }
            Edit::Insert('\n') => return,
            edit => match self.part {
                HeaderPart::Key => self.rows[self.selected].key.edit(edit),
                HeaderPart::Value => self.rows[self.selected].value.edit(edit),
            },
        }

        self.keep_one_empty_row();
    }

    pub fn next_field(&mut self) {
        match self.part {
            HeaderPart::Key => self.part = HeaderPart::Value,
            HeaderPart::Value => {
                self.part = HeaderPart::Key;
                self.selected = (self.selected + 1).min(self.rows.len() - 1);
            }
        }
    }

    pub fn previous_field(&mut self) {
        match self.part {
            HeaderPart::Value => self.part = HeaderPart::Key,
            HeaderPart::Key if self.selected > 0 => {
                self.selected -= 1;
                self.part = HeaderPart::Value;
            }
            HeaderPart::Key => {}
        }
    }

    fn add(&mut self, key: &str, value: &str) {
        let draft_index = self.rows.len() - 1;
        self.rows.insert(draft_index, HeaderRow::new(key, value));
    }

    fn values(&self) -> Result<Vec<(String, String)>, EzcurlError> {
        self.rows
            .iter()
            .filter(|row| !row.is_empty())
            .map(|row| {
                let key = row.key().trim();
                if key.is_empty() {
                    return Err(EzcurlError::InvalidHeader(
                        "header name cannot be empty".to_string(),
                    ));
                }

                Ok((key.to_string(), row.value().trim().to_string()))
            })
            .collect()
    }

    fn keep_one_empty_row(&mut self) {
        while self.rows.len() > 1
            && self.rows.last().is_some_and(HeaderRow::is_empty)
            && self.rows[self.rows.len() - 2].is_empty()
        {
            self.rows.pop();
            self.selected = self.selected.min(self.rows.len() - 1);
        }

        if self.rows.last().is_none_or(|row| !row.is_empty()) {
            self.rows.push(HeaderRow::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HeaderEditor, HeaderPart, HttpMethod};
    use crate::editor::Edit;

    #[test]
    fn typing_a_header_automatically_creates_the_next_row() {
        let mut headers = HeaderEditor::default();

        headers.edit(Edit::Insert('X'));

        assert_eq!(headers.rows().len(), 2);
        assert_eq!(headers.rows()[0].key(), "X");
        assert!(headers.rows()[1].is_empty());
    }

    #[test]
    fn advancing_moves_from_key_to_value_then_next_row() {
        let mut headers = HeaderEditor::default();
        headers.edit(Edit::Insert('X'));

        headers.next_field();
        assert_eq!(headers.part(), HeaderPart::Value);

        headers.next_field();
        assert_eq!(headers.part(), HeaderPart::Key);
        assert_eq!(headers.selected(), 1);
    }

    #[test]
    fn empty_draft_is_not_sent() {
        let mut headers = HeaderEditor::default();
        headers.add("Accept", "application/json");

        assert_eq!(
            headers.values().unwrap(),
            vec![("Accept".to_string(), "application/json".to_string())]
        );
    }

    #[test]
    fn arrows_cross_the_key_value_boundary() {
        let mut headers = HeaderEditor::default();
        headers.add("Accept", "application/json");

        headers.edit(Edit::Move(crate::action::Direction::Right));
        assert_eq!(headers.part(), HeaderPart::Value);
        assert_eq!(headers.active_editor().cursor_position(), (0, 0));

        headers.edit(Edit::Move(crate::action::Direction::Left));
        assert_eq!(headers.part(), HeaderPart::Key);
        assert_eq!(headers.active_editor().cursor_position(), (6, 0));
    }

    #[test]
    fn method_selection_wraps_around_the_list() {
        assert_eq!(HttpMethod::Options.next(), HttpMethod::Get);
        assert_eq!(HttpMethod::Get.previous(), HttpMethod::Options);
    }

    #[test]
    fn header_state_distinguishes_included_pending_and_invalid_rows() {
        let mut headers = HeaderEditor::default();
        assert_eq!(headers.rows()[0].state(), super::HeaderState::Pending);

        headers.select_part(HeaderPart::Value);
        headers.edit(Edit::Insert('x'));
        assert_eq!(headers.rows()[0].state(), super::HeaderState::Invalid);

        headers.select_part(HeaderPart::Key);
        headers.edit(Edit::Insert('X'));
        assert_eq!(headers.rows()[0].state(), super::HeaderState::Included);
    }
}
