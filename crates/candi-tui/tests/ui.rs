// SPDX-License-Identifier: AGPL-3.0

use std::sync::atomic::{AtomicUsize, Ordering};

use candi_core::ViewState;
use candi_pdf::{Document, Error};
use candi_tui::{App, Mode, Quit, RunError, draw, handle_key, run};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

struct FakeDoc {
    pages: Vec<String>,
    fail_page: Option<usize>,
    page_text_calls: AtomicUsize,
}

impl FakeDoc {
    fn new(pages: Vec<&str>) -> Self {
        Self {
            pages: pages.into_iter().map(str::to_string).collect(),
            fail_page: None,
            page_text_calls: AtomicUsize::new(0),
        }
    }

    fn failing(page: usize) -> Self {
        Self {
            pages: vec!["ok".into(), "boom".into(), "ok".into()],
            fail_page: Some(page),
            page_text_calls: AtomicUsize::new(0),
        }
    }
}

impl Document for FakeDoc {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        self.page_text_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_page == Some(page) {
            return Err(Error::Malformed("boom".into()));
        }
        Ok(self.pages[page].clone())
    }

    fn page_positions(&self, _page: usize) -> Result<Option<candi_pdf::PagePositions>, Error> {
        Ok(None)
    }

    fn page_size(&self, _page: usize) -> Result<(f32, f32), Error> {
        Ok((612.0, 792.0))
    }

    fn render_page(&self, _page: usize, _scale: f32) -> Result<candi_pdf::PageImage, Error> {
        Err(Error::Unsupported("test double cannot render".into()))
    }

    fn outline(&self) -> Result<Vec<candi_pdf::TocItem>, Error> {
        Ok(Vec::new())
    }
}

fn key_char(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty())
}

fn setup_app<'a>(doc: &'a FakeDoc, name: &str) -> App<'a, FakeDoc> {
    let mut app = App::new(doc, name, ViewState::new());
    app.resize(40, 10);
    app.reload_page().unwrap();
    app
}

fn buffer_text(app: &App<'_, FakeDoc>) -> String {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn status_shows_one_based_page() {
    let doc = FakeDoc::new(vec!["alpha", "beta", "gamma"]);
    let app = setup_app(&doc, "book.pdf");
    let text = buffer_text(&app);
    assert!(text.contains("book.pdf"));
    assert!(text.contains("1/3"));
}

#[test]
fn scroll_keys_change_offset_on_long_page() {
    let long = "word ".repeat(200);
    let doc = FakeDoc::new(vec![long.as_str()]);
    let mut app = setup_app(&doc, "long.pdf");
    let before = app.view().scroll_offset();
    handle_key(&mut app, key_char('j')).unwrap();
    assert!(app.view().scroll_offset() > before);
    handle_key(&mut app, key_char('k')).unwrap();
    assert_eq!(app.view().scroll_offset(), before);
}

#[test]
fn short_page_scroll_is_no_op() {
    let doc = FakeDoc::new(vec!["hi"]);
    let mut app = setup_app(&doc, "short.pdf");
    let before = app.view().scroll_offset();
    handle_key(&mut app, key_char('j')).unwrap();
    assert_eq!(app.view().scroll_offset(), before);
}

#[test]
fn page_keys_update_status() {
    let doc = FakeDoc::new(vec!["one", "two", "three"]);
    let mut app = setup_app(&doc, "pages.pdf");
    handle_key(&mut app, key_char('l')).unwrap();
    assert_eq!(app.view().page(), 1);
    assert!(buffer_text(&app).contains("2/3"));
    handle_key(&mut app, key_char('h')).unwrap();
    assert_eq!(app.view().page(), 0);
    assert!(buffer_text(&app).contains("1/3"));
}

#[test]
fn first_and_last_page_keys() {
    let doc = FakeDoc::new(vec!["a", "b", "c"]);
    let mut app = setup_app(&doc, "jump.pdf");
    handle_key(&mut app, key_char('G')).unwrap();
    assert_eq!(app.view().page(), 2);
    assert!(buffer_text(&app).contains("3/3"));
    handle_key(&mut app, key_char('g')).unwrap();
    assert_eq!(app.view().page(), 0);
    assert!(buffer_text(&app).contains("1/3"));
}

#[test]
fn search_prompt_submit_shows_query() {
    let doc = FakeDoc::new(vec!["foo bar", "bar baz", "qux"]);
    let mut app = setup_app(&doc, "find.pdf");
    handle_key(&mut app, key_char('/')).unwrap();
    assert!(matches!(app.mode(), Mode::Searching { .. }));
    for ch in "foo".chars() {
        handle_key(&mut app, key_char(ch)).unwrap();
    }
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    )
    .unwrap();
    assert!(matches!(app.mode(), Mode::Reading));
    assert!(app.search().is_some());
    let text = buffer_text(&app);
    assert!(text.contains("search: foo"));
}

#[test]
fn search_next_and_prev_navigate() {
    let doc = FakeDoc::new(vec!["foo", "bar foo", "foo again"]);
    let mut app = setup_app(&doc, "hits.pdf");
    handle_key(&mut app, key_char('/')).unwrap();
    for ch in "foo".chars() {
        handle_key(&mut app, key_char(ch)).unwrap();
    }
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    )
    .unwrap();
    handle_key(&mut app, key_char('n')).unwrap();
    assert_eq!(app.view().page(), 0);
    handle_key(&mut app, key_char('n')).unwrap();
    assert_eq!(app.view().page(), 1);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT),
    )
    .unwrap();
    assert_eq!(app.view().page(), 0);
}

#[test]
fn quit_key_sets_quit() {
    let doc = FakeDoc::new(vec!["x"]);
    let mut app = setup_app(&doc, "q.pdf");
    let quit = handle_key(&mut app, key_char('q')).unwrap();
    assert_eq!(quit, Some(Quit));
    assert!(app.should_quit());
}

#[test]
fn error_mode_shows_message() {
    let doc = FakeDoc::failing(0);
    let app = setup_app(&doc, "bad.pdf");
    assert!(matches!(app.mode(), Mode::Error { .. }));
    let text = buffer_text(&app);
    assert!(text.contains("malformed document"));
}

#[test]
fn error_mode_quit_key_exits() {
    let doc = FakeDoc::failing(0);
    let mut app = setup_app(&doc, "bad.pdf");
    assert!(matches!(app.mode(), Mode::Error { .. }));
    let quit = handle_key(&mut app, key_char('q')).unwrap();
    assert_eq!(quit, Some(Quit));
    assert!(app.should_quit());
}

#[test]
fn term_dumb_returns_terminal_error() {
    let prev = std::env::var("TERM").ok();
    unsafe {
        std::env::set_var("TERM", "dumb");
    }
    let doc = Box::new(FakeDoc::new(vec!["x"]));
    let result = run(doc, "test.pdf", ViewState::new());
    unsafe {
        match prev {
            Some(term) => std::env::set_var("TERM", term),
            None => std::env::remove_var("TERM"),
        }
    }
    assert!(matches!(result, Err(RunError::Terminal(_))));
}

#[test]
fn unmapped_reading_key_is_no_op() {
    let doc = FakeDoc::new(vec!["steady"]);
    let mut app = setup_app(&doc, "noop.pdf");
    let page = app.view().page();
    let scroll = app.view().scroll_offset();
    handle_key(&mut app, key_char('x')).unwrap();
    assert_eq!(app.view().page(), page);
    assert_eq!(app.view().scroll_offset(), scroll);
}

#[test]
fn resize_rewraps_without_panic() {
    let doc = FakeDoc::new(vec!["one two three four five six seven"]);
    let mut app = setup_app(&doc, "resize.pdf");
    app.resize(10, 8);
    app.reload_page().unwrap();
    let narrow = buffer_text(&app);
    app.resize(40, 8);
    app.reload_page().unwrap();
    let wide = buffer_text(&app);
    assert_ne!(narrow, wide);
}

#[test]
fn scroll_down_at_page_end_advances_via_key() {
    let long = "word ".repeat(200);
    let doc = FakeDoc::new(vec![long.as_str(), "page two"]);
    let mut app = setup_app(&doc, "pages.pdf");
    loop {
        let page = app.view().page();
        handle_key(&mut app, key_char('j')).unwrap();
        if app.view().page() != page {
            break;
        }
    }
    assert_eq!(app.view().page(), 1);
    assert_eq!(app.view().scroll_offset(), 0);
}

#[test]
fn scroll_up_at_page_top_retreats_via_key() {
    let long = "word ".repeat(200);
    let doc = FakeDoc::new(vec![long.as_str(), "page two"]);
    let mut app = setup_app(&doc, "pages.pdf");
    handle_key(&mut app, key_char('l')).unwrap();
    assert_eq!(app.view().page(), 1);
    assert_eq!(app.view().scroll_offset(), 0);
    handle_key(&mut app, key_char('k')).unwrap();
    assert_eq!(app.view().page(), 0);
    assert!(app.view().scroll_offset() > 0);
}

#[test]
fn draw_fills_background_on_test_backend() {
    let doc = FakeDoc::new(vec!["hi"]);
    let app = setup_app(&doc, "bg.pdf");
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(frame, &app)).unwrap();
    let main_bg = ratatui::style::Color::Rgb(18, 18, 22);
    let status_bg = ratatui::style::Color::Rgb(30, 30, 36);
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .all(|cell| { cell.bg == main_bg || cell.bg == status_bg })
    );
}

#[test]
fn search_esc_cancels_without_session() {
    let doc = FakeDoc::new(vec!["text"]);
    let mut app = setup_app(&doc, "cancel.pdf");
    handle_key(&mut app, key_char('/')).unwrap();
    handle_key(&mut app, key_char('z')).unwrap();
    handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())).unwrap();
    assert!(matches!(app.mode(), Mode::Reading));
    assert!(app.search().is_none());
}
