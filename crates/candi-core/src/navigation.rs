// SPDX-License-Identifier: AGPL-3.0

/// View position within a document: current page and vertical scroll offset.
///
/// Page indices are **0-based** in core (the TUI displays them 1-based).
/// All operations clamp out-of-range inputs; empty documents (`page_count == 0`)
/// never panic.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewState {
    page: usize,
    scroll_offset: usize,
}

impl ViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn page(self) -> usize {
        self.page
    }

    pub fn scroll_offset(self) -> usize {
        self.scroll_offset
    }

    pub fn first_page(self, page_count: usize) -> Self {
        self.with_page(0, page_count)
    }

    pub fn last_page(self, page_count: usize) -> Self {
        let page = last_page_index(page_count);
        self.with_page(page, page_count)
    }

    pub fn next_page(self, page_count: usize) -> Self {
        let page = (self.page + 1).min(last_page_index(page_count));
        self.with_page(page, page_count)
    }

    pub fn prev_page(self, page_count: usize) -> Self {
        let page = self.page.saturating_sub(1);
        self.with_page(page, page_count)
    }

    pub fn scroll_down(self, delta: usize, max_scroll: usize) -> Self {
        let scroll_offset = (self.scroll_offset + delta).min(max_scroll);
        Self {
            page: self.page,
            scroll_offset,
        }
    }

    pub fn scroll_up(self, delta: usize, max_scroll: usize) -> Self {
        let scroll_offset = self.scroll_offset.saturating_sub(delta).min(max_scroll);
        Self {
            page: self.page,
            scroll_offset,
        }
    }

    fn with_page(self, page: usize, page_count: usize) -> Self {
        Self {
            page: page.min(last_page_index(page_count)),
            scroll_offset: 0,
        }
    }
}

fn last_page_index(page_count: usize) -> usize {
    page_count.saturating_sub(1)
}
