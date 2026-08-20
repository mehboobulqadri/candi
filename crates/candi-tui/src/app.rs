// SPDX-License-Identifier: AGPL-3.0

use candi_core::{SearchSession, ViewState};
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

        let page = self.view.page().min(page_count - 1);
        match self.document.page_text(page) {
            Ok(text) => {
                self.page_text = text;
                self.recompute_wrap();
                self.view = self
                    .view
                    .scroll_down(0, self.max_scroll)
                    .scroll_up(0, self.max_scroll);
                Ok(())
            }
            Err(err) => {
                self.enter_error(err);
                Ok(())
            }
        }
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
            Action::ScrollDown => {
                self.view = self.view.scroll_down(1, self.max_scroll);
            }
            Action::ScrollUp => {
                self.view = self.view.scroll_up(1, self.max_scroll);
            }
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

    fn enter_error(&mut self, err: Error) {
        self.mode = Mode::Error {
            message: err.to_string(),
        };
    }

    fn recompute_wrap(&mut self) {
        let wrap_width = usize::from(self.width.max(1));
        self.wrapped_lines = wrap_lines(&self.page_text, wrap_width);
        self.max_scroll = self
            .wrapped_lines
            .len()
            .saturating_sub(usize::from(self.viewport_rows));
    }
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
            if current.len() + 1 + word.len() <= width {
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
        assert!(lines.iter().all(|line| line.len() <= 8));
        assert!(lines.len() > 1);
    }
}
