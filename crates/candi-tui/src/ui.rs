// SPDX-License-Identifier: AGPL-3.0

use candi_pdf::Document;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, Mode};

pub fn draw<D: Document + ?Sized>(frame: &mut Frame, app: &App<'_, D>) {
    match app.mode() {
        Mode::Error { message } => draw_error(frame, message),
        Mode::Searching { draft } => {
            draw_reading(frame, app);
            draw_search_overlay(frame, draft);
        }
        Mode::Reading => draw_reading(frame, app),
    }
}

fn draw_reading<D: Document + ?Sized>(frame: &mut Frame, app: &App<'_, D>) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let visible: Vec<Line> = app
        .wrapped_lines()
        .iter()
        .skip(app.view().scroll_offset())
        .take(usize::from(app.viewport_rows()))
        .map(|line| Line::from(line.as_str()))
        .collect();

    frame.render_widget(Paragraph::new(visible), chunks[0]);
    frame.render_widget(
        Paragraph::new(Line::from(status_text(app))).style(Style::default().reversed()),
        chunks[1],
    );
}

fn draw_search_overlay(frame: &mut Frame, draft: &str) {
    let area = centered_rect(60, 3, frame.area());
    let prompt = format!("/{draft}");
    frame.render_widget(
        Paragraph::new(prompt).block(Block::bordered().title("search")),
        area,
    );
}

fn draw_error(frame: &mut Frame, message: &str) {
    let text = format!("Error: {message}\n\nPress q to quit.");
    frame.render_widget(
        Paragraph::new(text).block(Block::bordered().title("candi")),
        frame.area(),
    );
}

fn status_text<D: Document + ?Sized>(app: &App<'_, D>) -> String {
    let page_count = app.document().page_count();
    let page = app.view().page().min(page_count.saturating_sub(1));
    let mut status = format!(" {}  {}/{} ", app.filename(), page + 1, page_count.max(1));

    if let Some(session) = app.search() {
        let hits = session.results().len();
        if hits == 0 {
            status.push_str(&format!("  search: {} (no hits)", session.query()));
        } else if let Some(current) = session.current() {
            let index = session
                .results()
                .iter()
                .position(|item| *item == current)
                .map(|idx| idx + 1)
                .unwrap_or(1);
            status.push_str(&format!(
                "  search: {} ({}/{})",
                session.query(),
                index,
                hits
            ));
        } else {
            status.push_str(&format!("  search: {} ({} hits)", session.query(), hits));
        }
    }

    status
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
