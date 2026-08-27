// SPDX-License-Identifier: AGPL-3.0

use candi_pdf::Document;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::{App, Mode, reader_column_width};

const BG: Color = Color::Rgb(18, 18, 22);
const FG: Color = Color::Rgb(230, 230, 225);
const STATUS_BG: Color = Color::Rgb(30, 30, 36);

fn base_style() -> Style {
    Style::default().bg(BG).fg(FG)
}

fn status_style() -> Style {
    Style::default().bg(STATUS_BG).fg(FG)
}

fn paint_background(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(base_style()), area);
}

pub fn draw<D: Document + ?Sized>(frame: &mut Frame, app: &App<'_, D>) {
    paint_background(frame, frame.area());
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

    let text_area = reading_column(chunks[0]);

    let visible: Vec<Line> = app
        .wrapped_lines()
        .iter()
        .skip(app.view().scroll_offset())
        .take(usize::from(app.viewport_rows()))
        .map(|line| Line::from(line.as_str()))
        .collect();

    frame.render_widget(Paragraph::new(visible).style(base_style()), text_area);
    frame.render_widget(
        Paragraph::new(Line::from(status_text(app))).style(status_style()),
        chunks[1],
    );
}

fn draw_search_overlay(frame: &mut Frame, draft: &str) {
    let area = centered_rect(60, 3, frame.area());
    let prompt = format!("/{draft}");
    frame.render_widget(
        Paragraph::new(prompt)
            .style(base_style())
            .block(Block::bordered().title("search").style(base_style())),
        area,
    );
}

fn draw_error(frame: &mut Frame, message: &str) {
    let text = format!("Error: {message}\n\nPress q to quit.");
    frame.render_widget(
        Paragraph::new(text).block(Block::bordered().title("candi").style(base_style())),
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

fn reading_column(area: Rect) -> Rect {
    let col_width = reader_column_width(area.width) as u16;
    let x = area.x + (area.width.saturating_sub(col_width)) / 2;
    Rect {
        x,
        y: area.y,
        width: col_width.min(area.width),
        height: area.height,
    }
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
