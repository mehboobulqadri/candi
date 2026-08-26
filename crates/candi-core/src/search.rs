// SPDX-License-Identifier: AGPL-3.0

use candi_pdf::{Document, Error};

use crate::normalize_reader_text;

/// Lazy, page-at-a-time text search over any [`Document`].
///
/// Matching is case-insensitive via `to_lowercase()` on the query and each page
/// (ASCII-oriented; not full Unicode case folding). Hit offsets are UTF-8 byte
/// indices into the lowercased page string. Overlapping matches are skipped
/// (advance by needle length, minimum one byte).
pub struct SearchSession<'a, D: Document + ?Sized> {
    document: &'a D,
    query: String,
    start_page: usize,
    results: Vec<(usize, usize)>,
    cursor: Option<usize>,
    scan: ScanState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanState {
    Active { next_page: usize, wrapped: bool },
    Complete,
}

impl<'a, D: Document + ?Sized> SearchSession<'a, D> {
    pub fn new(document: &'a D, query: impl Into<String>, start_page: usize) -> Self {
        let query = query.into().to_lowercase();
        let page_count = document.page_count();
        let start_page = start_page.min(last_page_index(page_count));
        let scan = if query.is_empty() || page_count == 0 {
            ScanState::Complete
        } else {
            ScanState::Active {
                next_page: start_page,
                wrapped: false,
            }
        };
        Self {
            document,
            query,
            start_page,
            results: Vec::new(),
            cursor: None,
            scan,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<(usize, usize)>, Error> {
        if self.query.is_empty() {
            return Ok(None);
        }

        if let Some(idx) = self.cursor {
            if idx + 1 < self.results.len() {
                self.cursor = Some(idx + 1);
                return Ok(Some(self.results[idx + 1]));
            }
            if !self.scan_complete() {
                while !self.scan_complete() {
                    let before = self.results.len();
                    self.scan_one_page()?;
                    if self.results.len() > before {
                        self.cursor = Some(idx + 1);
                        return Ok(Some(self.results[idx + 1]));
                    }
                }
            }
            if self.results.is_empty() {
                return Ok(None);
            }
            self.cursor = Some(0);
            return Ok(Some(self.results[0]));
        }

        while !self.scan_complete() {
            self.scan_one_page()?;
            if !self.results.is_empty() {
                self.cursor = Some(0);
                return Ok(Some(self.results[0]));
            }
        }
        Ok(None)
    }

    pub fn prev(&mut self) -> Result<Option<(usize, usize)>, Error> {
        if self.query.is_empty() {
            return Ok(None);
        }

        if let Some(idx) = self.cursor {
            if idx > 0 {
                self.cursor = Some(idx - 1);
                return Ok(Some(self.results[idx - 1]));
            }
            if !self.scan_complete() {
                while !self.scan_complete() {
                    self.scan_one_page()?;
                }
            }
            if self.results.is_empty() {
                return Ok(None);
            }
            let last = self.results.len() - 1;
            self.cursor = Some(last);
            return Ok(Some(self.results[last]));
        }

        while !self.scan_complete() {
            self.scan_one_page()?;
        }
        if self.results.is_empty() {
            return Ok(None);
        }
        let last = self.results.len() - 1;
        self.cursor = Some(last);
        Ok(Some(self.results[last]))
    }

    pub fn current(&self) -> Option<(usize, usize)> {
        self.cursor.map(|idx| self.results[idx])
    }

    pub fn results(&self) -> &[(usize, usize)] {
        &self.results
    }

    fn scan_complete(&self) -> bool {
        matches!(self.scan, ScanState::Complete)
    }

    fn scan_one_page(&mut self) -> Result<(), Error> {
        let ScanState::Active {
            mut next_page,
            mut wrapped,
        } = self.scan
        else {
            return Ok(());
        };

        let page_count = self.document.page_count();
        let text = self.document.page_text(next_page)?;
        let haystack = normalize_reader_text(&text).to_lowercase();
        let needle = &self.query;
        let needle_len = needle.len();

        let mut start = 0;
        while start + needle_len <= haystack.len() {
            if haystack.is_char_boundary(start)
                && haystack.as_bytes()[start..].starts_with(needle.as_bytes())
            {
                self.results.push((next_page, start));
                start += needle_len.max(1);
            } else {
                start += 1;
            }
        }

        if next_page + 1 < page_count {
            next_page += 1;
        } else if !wrapped && self.start_page > 0 {
            wrapped = true;
            next_page = 0;
        } else {
            self.scan = ScanState::Complete;
            return Ok(());
        }

        if wrapped && next_page >= self.start_page {
            self.scan = ScanState::Complete;
        } else {
            self.scan = ScanState::Active { next_page, wrapped };
        }
        Ok(())
    }
}

fn last_page_index(page_count: usize) -> usize {
    page_count.saturating_sub(1)
}
