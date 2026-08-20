// SPDX-License-Identifier: AGPL-3.0

use candi_core::{SearchSession, ViewState, normalize_reader_text};
use candi_pdf::{Document, Error};

use crate::keymap::Action;
use crate::search::SearchInput;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Reading,
    Searching { draft: String },
    Error { message: String },
}

pub struct App<'a, D: Document + ?Sized> {
    document: &'a D,
    filename: String,
    view: ViewState,
    mode: Mode,
    search: Option<SearchSession<'a, D>>,
    page_text: String,
    wrapped_lines: Vec<String>,
    max_scroll: usize,
    viewport_rows: u16,
    width: u16,
    quit: bool,
}

impl<'a, D: Document + ?Sized> App<'a, D> {
    pub fn new(document: &'a D, filename: impl Into<String>, view: ViewState) -> Self {
        Self {
            document,
            filename: filename.into(),
            view,
            mode: Mode::Reading,
            search: None,
            page_text: String::new(),
            wrapped_lines: Vec::new(),
            max_scroll: 0,
            viewport_rows: 1,
            width: 80,
            quit: false,
        }
    }

    pub fn document(&self) -> &D {
        self.document
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn view(&self) -> ViewState {
        self.view
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn search(&self) -> Option<&SearchSession<'a, D>> {
        self.search.as_ref()
    }

    pub fn wrapped_lines(&self) -> &[String] {
        &self.wrapped_lines
    }

    pub fn viewport_rows(&self) -> u16 {
        self.viewport_rows
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn set_error(&mut self, message: String) {
        self.mode = Mode::Error { message };
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width.max(1);
        self.viewport_rows = height.saturating_sub(1).max(1);
        self.recompute_wrap();
        self.view = self
            .view
            .scroll_down(0, self.max_scroll)
            .scroll_up(0, self.max_scroll);
    }

    pub fn reload_page(&mut self) -> Result<(), Error> {
        let page_count = self.document.page_count();
        if page_count == 0 {
            self.enter_error(Error::Malformed("document has no pages".into()));
            return Ok(());
        }

        let start = self.view.page().min(page_count - 1);
        let (page, text) = match self.first_non_empty_from(start, page_count) {
            Ok(found) => found,
            Err(err) => {
                self.enter_error(err);
                return Ok(());
            }
        };

        if page != self.view.page() {
            self.view = self.view.goto_page(page, page_count);
        }
        self.page_text = normalize_reader_text(&text);
        self.recompute_wrap();
        self.view = self
            .view
            .scroll_down(0, self.max_scroll)
            .scroll_up(0, self.max_scroll);
        Ok(())
    }

    pub fn apply_action(&mut self, action: Action) -> Result<(), Error> {
        if matches!(self.mode, Mode::Error { .. }) {
            if action == Action::Quit {
                self.quit = true;
            }
            return Ok(());
        }

        match action {
            Action::Quit => self.quit = true,
            Action::None => {}
            Action::EnterSearch => {
                self.mode = Mode::Searching {
                    draft: String::new(),
                };
            }
            Action::ScrollDown => self.scroll_down()?,
            Action::ScrollUp => self.scroll_up()?,
            Action::NextPage => {
                self.view = self.view.next_page(self.document.page_count());
                self.reload_page()?;
            }
            Action::PrevPage => {
                self.view = self.view.prev_page(self.document.page_count());
                self.reload_page()?;
            }
            Action::FirstPage => {
                self.view = self.view.first_page(self.document.page_count());
                self.reload_page()?;
            }
            Action::LastPage => {
                self.view = self.view.last_page(self.document.page_count());
                self.reload_page()?;
            }
            Action::SearchNext => {
                if matches!(self.mode, Mode::Reading) {
                    self.search_next()?;
                }
            }
            Action::SearchPrev => {
                if matches!(self.mode, Mode::Reading) {
                    self.search_prev()?;
                }
            }
        }
        Ok(())
    }

    pub fn scroll_down(&mut self) -> Result<(), Error> {
        if !matches!(self.mode, Mode::Reading) {
            return Ok(());
        }
        let page_count = self.document.page_count();
        if page_count == 0 {
            return Ok(());
        }
        let last = page_count - 1;
        if self.view.scroll_offset() >= self.max_scroll && self.view.page() < last {
            self.view = self.view.next_page(page_count);
            self.reload_page()?;
        } else {
            self.view = self.view.scroll_down(1, self.max_scroll);
        }
        Ok(())
    }

    pub fn scroll_up(&mut self) -> Result<(), Error> {
        if !matches!(self.mode, Mode::Reading) {
            return Ok(());
        }
        let page_count = self.document.page_count();
        if page_count == 0 {
            return Ok(());
        }
        if self.view.scroll_offset() == 0 && self.view.page() > 0 {
            self.view = self.view.prev_page(page_count);
            self.reload_page()?;
            self.view = self.view.scroll_down(self.max_scroll, self.max_scroll);
        } else {
            self.view = self.view.scroll_up(1, self.max_scroll);
        }
        Ok(())
    }

    pub fn apply_search_input(&mut self, input: SearchInput) -> Result<(), Error> {
        let Mode::Searching { draft } = &mut self.mode else {
            return Ok(());
        };

        match input {
            SearchInput::Append(ch) => draft.push(ch),
            SearchInput::Backspace => {
                draft.pop();
            }
            SearchInput::Cancel => self.mode = Mode::Reading,
            SearchInput::Submit => {
                let query = std::mem::take(draft);
                self.mode = Mode::Reading;
                self.search = Some(SearchSession::new(self.document, query, self.view.page()));
            }
        }
        Ok(())
    }

    fn search_next(&mut self) -> Result<(), Error> {
        let Some(session) = self.search.as_mut() else {
            return Ok(());
        };
        if let Some((page, _)) = session.next()? {
            self.jump_to_page(page)?;
        }
        Ok(())
    }

    fn search_prev(&mut self) -> Result<(), Error> {
        let Some(session) = self.search.as_mut() else {
            return Ok(());
        };
        if let Some((page, _)) = session.prev()? {
            self.jump_to_page(page)?;
        }
        Ok(())
    }

    fn jump_to_page(&mut self, target: usize) -> Result<(), Error> {
        let page_count = self.document.page_count();
        let current = self.view.page();
        if target > current {
            for _ in current..target {
                self.view = self.view.next_page(page_count);
            }
        } else {
            for _ in target..current {
                self.view = self.view.prev_page(page_count);
            }
        }
        self.reload_page()
    }

    fn first_non_empty_from(
        &self,
        start: usize,
        page_count: usize,
    ) -> Result<(usize, String), Error> {
        let start = start.min(page_count - 1);
        for offset in 0..page_count {
            let page = start.saturating_add(offset);
            if page >= page_count {
                break;
            }
            let text = self.document.page_text(page)?;
            if !text.trim().is_empty() {
                return Ok((page, text));
            }
        }
        Ok((start, self.document.page_text(start)?))
    }

    fn enter_error(&mut self, err: Error) {
        self.mode = Mode::Error {
            message: err.to_string(),
        };
    }

    fn recompute_wrap(&mut self) {
        let wrap_width = reader_column_width(self.width);
        self.wrapped_lines = wrap_lines(&self.page_text, wrap_width);
        self.max_scroll = self
            .wrapped_lines
            .len()
            .saturating_sub(usize::from(self.viewport_rows));
    }
}

pub(crate) fn reader_column_width(width: u16) -> usize {
    72.min(usize::from(width.saturating_sub(4))).max(1)
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = words[0].to_string();
        for word in &words[1..] {
            if char_len(&current) + 1 + char_len(word) <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(current);
                current = (*word).to_string();
            }
        }
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_empty_paragraph() {
        let lines = wrap_lines("hello\n\nworld", 80);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "");
    }

    #[test]
    fn wrap_breaks_long_line() {
        let lines = wrap_lines("one two three four", 8);
        assert!(lines.iter().all(|line| char_len(line) <= 8));
        assert!(lines.len() > 1);
    }

    #[test]
    fn skip_blank_first_page() {
        let doc = BlankFirstPageDoc;
        let mut app = App::new(&doc, "blank.pdf", ViewState::new());
        app.resize(40, 10);
        app.reload_page().unwrap();
        assert_eq!(app.view().page(), 1);
        assert!(
            app.wrapped_lines()
                .iter()
                .any(|line| line.contains("Page two"))
        );
    }

    #[test]
    fn normalize_ligatures_on_load() {
        let doc = LigatureDoc;
        let mut app = App::new(&doc, "lig.pdf", ViewState::new());
        app.resize(40, 10);
        app.reload_page().unwrap();
        assert!(app.wrapped_lines().join(" ").contains("finger"));
    }

    #[test]
    fn scroll_down_at_page_end_advances() {
        let long = "word ".repeat(200);
        let doc = TwoPageDoc {
            pages: [long, "second page".into()],
        };
        let mut app = App::new(&doc, "two.pdf", ViewState::new());
        app.resize(40, 10);
        app.reload_page().unwrap();
        assert_eq!(app.view().page(), 0);
        loop {
            let page = app.view().page();
            app.scroll_down().unwrap();
            if app.view().page() != page {
                break;
            }
        }
        assert_eq!(app.view().page(), 1);
        assert_eq!(app.view().scroll_offset(), 0);
    }

    #[test]
    fn scroll_up_at_page_top_retreats_to_prev_end() {
        let long = "word ".repeat(200);
        let doc = TwoPageDoc {
            pages: [long, "second page".into()],
        };
        let mut app = App::new(&doc, "two.pdf", ViewState::new());
        app.resize(40, 10);
        app.reload_page().unwrap();
        app.view = app.view.goto_page(1, doc.page_count());
        app.reload_page().unwrap();
        assert_eq!(app.view().page(), 1);
        assert_eq!(app.view().scroll_offset(), 0);
        app.scroll_up().unwrap();
        assert_eq!(app.view().page(), 0);
        assert!(app.view().scroll_offset() > 0);
    }

    struct TwoPageDoc {
        pages: [String; 2],
    }

    impl Document for TwoPageDoc {
        fn page_count(&self) -> usize {
            2
        }

        fn page_text(&self, page: usize) -> Result<String, Error> {
            self.pages
                .get(page)
                .cloned()
                .ok_or_else(|| Error::Malformed("bad page".into()))
        }

        fn page_positions(&self, _page: usize) -> Result<Option<candi_pdf::PagePositions>, Error> {
            Ok(None)
        }
    }

    struct BlankFirstPageDoc;

    impl Document for BlankFirstPageDoc {
        fn page_count(&self) -> usize {
            2
        }

        fn page_text(&self, page: usize) -> Result<String, Error> {
            match page {
                0 => Ok(String::new()),
                1 => Ok("Page two text".into()),
                _ => Err(Error::Malformed("bad page".into())),
            }
        }

        fn page_positions(&self, _page: usize) -> Result<Option<candi_pdf::PagePositions>, Error> {
            Ok(None)
        }
    }

    struct LigatureDoc;

    impl Document for LigatureDoc {
        fn page_count(&self) -> usize {
            1
        }

        fn page_text(&self, _page: usize) -> Result<String, Error> {
            Ok(format!("{}nger", '\u{fb01}'))
        }

        fn page_positions(&self, _page: usize) -> Result<Option<candi_pdf::PagePositions>, Error> {
            Ok(None)
        }
    }
}
