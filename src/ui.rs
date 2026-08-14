use crate::{
    app::{App, AppMode, Panel},
    request::{HeaderPart, HeaderState, HttpMethod, RequestField},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

struct RequestAreas {
    method: Rect,
    url: Rect,
    headers: [Rect; 2],
    body: Rect,
}

fn panel_style(app: &App, panel: Panel) -> Style {
    if !app.history_open() && app.focused_panel() == panel {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn panel_border_type(app: &App, panel: Panel) -> BorderType {
    if !app.history_open() && app.focused_panel() == panel {
        BorderType::Thick
    } else {
        BorderType::Plain
    }
}

fn header_part_style(app: &App, part: HeaderPart) -> Style {
    if !app.history_open()
        && app.focused_panel() == Panel::Headers
        && app.request().header_editor().part() == part
    {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let page = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    let (request_area, response_area) = if app.history_open() {
        let columns = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(38),
            Constraint::Percentage(37),
        ])
        .split(page[0]);
        render_history(frame, app, columns[0]);
        (columns[1], columns[2])
    } else {
        let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(page[0]);
        (columns[0], columns[1])
    };

    let request_areas = render_request(frame, app, request_area);
    render_response(frame, app, response_area);
    render_footer(frame, app, page[1]);

    if app.mode() == AppMode::Insert {
        show_cursor(frame, app, &request_areas);

        if app.focused_panel() == Panel::Method {
            render_method_menu(
                frame,
                app.request().method(),
                request_areas.method,
                request_area,
            );
        }
    }
}

fn render_request(frame: &mut Frame, app: &App, area: Rect) -> RequestAreas {
    let request = app.displayed_request();
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Percentage(35),
        Constraint::Min(5),
    ])
    .split(area);
    let request_bar =
        Layout::horizontal([Constraint::Length(12), Constraint::Min(1)]).split(rows[0]);
    let method_area = request_bar[0];
    let url_area = request_bar[1];
    let headers_area = rows[1];
    let body_area = rows[2];

    frame.render_widget(
        Paragraph::new(request.method().as_str()).block(
            Block::default()
                .title("METHOD")
                .borders(Borders::ALL)
                .border_style(panel_style(app, Panel::Method))
                .border_type(panel_border_type(app, Panel::Method)),
        ),
        method_area,
    );
    frame.render_widget(
        Paragraph::new(request.url()).block(
            Block::default()
                .title("URL")
                .borders(Borders::ALL)
                .border_style(panel_style(app, Panel::Url))
                .border_type(panel_border_type(app, Panel::Url)),
        ),
        url_area,
    );

    let headers_block = Block::default()
        .title("HEADERS")
        .borders(Borders::ALL)
        .border_style(panel_style(app, Panel::Headers))
        .border_type(panel_border_type(app, Panel::Headers));
    let headers_inner = headers_block.inner(headers_area);
    frame.render_widget(headers_block, headers_area);
    let header_columns =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(headers_inner);

    let header_keys = request
        .header_editor()
        .rows()
        .iter()
        .map(|row| {
            let (indicator, style) = match row.state() {
                HeaderState::Included => ("[x]", Style::default().fg(Color::Green)),
                HeaderState::Pending => ("[ ]", Style::default().fg(Color::DarkGray)),
                HeaderState::Invalid => ("[!]", Style::default().fg(Color::Red)),
            };
            Line::from(vec![
                Span::styled(indicator, style),
                Span::raw(format!(" {}", row.key())),
            ])
        })
        .collect::<Vec<_>>();
    let header_values = request
        .header_editor()
        .rows()
        .iter()
        .map(|row| Line::raw(row.value().to_string()))
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(Text::from(header_keys)).block(
            Block::default()
                .title("KEY")
                .borders(Borders::TOP | Borders::RIGHT)
                .border_style(header_part_style(app, HeaderPart::Key)),
        ),
        header_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Text::from(header_values)).block(
            Block::default()
                .title("VALUE")
                .borders(Borders::TOP)
                .border_style(header_part_style(app, HeaderPart::Value)),
        ),
        header_columns[1],
    );

    frame.render_widget(
        Paragraph::new(request.editor(RequestField::Body).text()).block(
            Block::default()
                .title("BODY")
                .borders(Borders::ALL)
                .border_style(panel_style(app, Panel::Body))
                .border_type(panel_border_type(app, Panel::Body)),
        ),
        body_area,
    );

    RequestAreas {
        method: method_area,
        url: url_area,
        headers: [header_columns[0], header_columns[1]],
        body: body_area,
    }
}

fn render_response(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(error) = app.displayed_response_error() {
        Text::from(vec![
            Line::styled("REQUEST FAILED", Style::default().fg(Color::Red)),
            Line::default(),
            Line::raw(error.to_string()),
        ])
    } else if let Some(response) = app.displayed_response() {
        let status = response.status();
        let status_color = match status {
            200..=299 => Color::Green,
            300..=399 => Color::Yellow,
            _ => Color::Red,
        };
        let mut headers = response.headers().iter().collect::<Vec<_>>();
        headers.sort_by_key(|(name, _)| *name);

        let mut lines = vec![
            Line::from(vec![
                Span::raw("STATUS  "),
                Span::styled(
                    status.to_string(),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::default(),
            Line::styled("HEADERS", Style::default().add_modifier(Modifier::BOLD)),
        ];
        lines.extend(
            headers
                .into_iter()
                .map(|(name, value)| Line::raw(format!("{name}: {value}"))),
        );
        lines.push(Line::default());
        lines.push(Line::styled(
            "BODY",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.extend(
            String::from_utf8_lossy(response.body())
                .lines()
                .map(|line| Line::raw(line.to_string())),
        );
        Text::from(lines)
    } else {
        Text::from("Ctrl+S pour envoyer la requete")
    };

    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: false }).block(
            Block::default()
                .title("RESPONSE")
                .borders(Borders::ALL)
                .border_style(panel_style(app, Panel::Response))
                .border_type(panel_border_type(app, Panel::Response)),
        ),
        area,
    );
}

fn render_history(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();
    if let Some(error) = app.history_storage_error() {
        lines.push(Line::styled(
            format!("Storage error: {error}"),
            Style::default().fg(Color::Red),
        ));
        lines.push(Line::default());
    }

    if app.history().is_empty() {
        lines.push(Line::raw("Aucune requete dans cette session"));
    } else {
        let visible_rows = area
            .height
            .saturating_sub(2)
            .saturating_sub(lines.len() as u16) as usize;
        let start = app
            .history_selected()
            .saturating_sub(visible_rows.saturating_sub(1));
        lines.extend(
            app.history()
                .iter()
                .enumerate()
                .skip(start)
                .take(visible_rows)
                .map(|(index, entry)| {
                    let outcome = entry
                        .response()
                        .map(|response| response.status().to_string())
                        .unwrap_or_else(|| {
                            if entry.error().is_some() {
                                "ERR".to_string()
                            } else {
                                "---".to_string()
                            }
                        });
                    let line = format!(
                        "{} {} {}",
                        entry.request().method().as_str(),
                        outcome,
                        entry.request().url()
                    );

                    if index == app.history_selected() {
                        Line::styled(
                            format!("> {line}"),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Line::raw(format!("  {line}"))
                    }
                }),
        );
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(
            Block::default()
                .title(format!("HISTORY ({})", app.history().len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        area,
    );
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let footer = Layout::horizontal([
        Constraint::Length(9),
        Constraint::Min(1),
        Constraint::Length(10),
    ])
    .split(area);

    let (mode, mode_style) = match app.mode() {
        AppMode::Normal => (
            " NORMAL ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        AppMode::Insert => (
            " INSERT ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };
    frame.render_widget(Paragraph::new(mode).style(mode_style), footer[0]);

    let controls = if app.leader_pending() {
        " leader: e historique  Esc annuler"
    } else if app.history_open() {
        " j/k choisir et previsualiser  Entree restaurer  Esc ou Espace+e fermer"
    } else if app.mode() == AppMode::Normal {
        if app.focused_panel() == Panel::Headers {
            " h/l key-value  j/k naviguer  i/Entree editer  [x] inclus [ ] nouveau [!] invalide"
        } else {
            " hjkl naviguer  i/Entree editer  Tab suivant  Ctrl+S envoyer  Espace+e historique  q quitter"
        }
    } else {
        match app.focused_panel() {
            Panel::Method => " j/k ou Haut/Bas choisir  Entree confirmer  Tab suivant  Esc normal",
            Panel::Headers => {
                " Fleches curseur/champ/ligne  Tab/Entree suivant  Shift+Tab precedent  Ctrl+S envoyer  Esc normal"
            }
            Panel::Body => {
                " Fleches curseur  Entree nouvelle ligne  Home/End  Tab suivant  Ctrl+S envoyer  Esc normal"
            }
            Panel::Url => {
                " Gauche/Droite curseur  Home/End  Entree confirmer  Tab suivant  Ctrl+S envoyer  Esc normal"
            }
            Panel::Response => " Esc normal",
        }
    };
    frame.render_widget(
        Paragraph::new(controls).style(Style::default().fg(Color::DarkGray)),
        footer[1],
    );
    frame.render_widget(
        Paragraph::new(" ezcurl ").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        footer[2],
    );
}

fn render_method_menu(frame: &mut Frame, selected: HttpMethod, anchor: Rect, bounds: Rect) {
    let y = anchor.bottom();
    let available_height = bounds.bottom().saturating_sub(y);
    let height = (HttpMethod::ALL.len() as u16 + 2).min(available_height);
    if height < 3 {
        return;
    }

    let area = Rect::new(
        anchor.x,
        y,
        anchor.width.min(bounds.right().saturating_sub(anchor.x)),
        height,
    );
    let visible_items = height.saturating_sub(2) as usize;
    let selected_index = HttpMethod::ALL
        .iter()
        .position(|method| *method == selected)
        .unwrap_or_default();
    let start = selected_index.saturating_sub(visible_items.saturating_sub(1));
    let lines = HttpMethod::ALL[start..]
        .iter()
        .take(visible_items)
        .map(|method| {
            if *method == selected {
                Line::styled(
                    format!("> {}", method.as_str()),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Line::raw(format!("  {}", method.as_str()))
            }
        })
        .collect::<Vec<_>>();

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(
            Block::default()
                .title("METHODS")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        area,
    );
}

fn show_cursor(frame: &mut Frame, app: &App, areas: &RequestAreas) {
    let Some(editor) = app.focused_editor() else {
        return;
    };
    let (cursor_x, cursor_y) = editor.cursor_position();

    let position = match app.focused_panel() {
        Panel::Url => (areas.url.x + 1 + cursor_x, areas.url.y + 1 + cursor_y),
        Panel::Headers => {
            let headers = app.request().header_editor();
            let area = match headers.part() {
                HeaderPart::Key => areas.headers[0],
                HeaderPart::Value => areas.headers[1],
            };
            let indicator_width = u16::from(headers.part() == HeaderPart::Key) * 4;
            (
                area.x + indicator_width + cursor_x,
                area.y + 1 + headers.selected() as u16,
            )
        }
        Panel::Body => (areas.body.x + 1 + cursor_x, areas.body.y + 1 + cursor_y),
        Panel::Method | Panel::Response => return,
    };

    frame.set_cursor_position(position);
}
