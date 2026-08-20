// SPDX-License-Identifier: AGPL-3.0

//! Scriptable test double implementing the full trait surface.
//!
//! The parity suite (01/04) and core tests (01/06–08) build on this; every
//! behavior a backend can exhibit is scriptable here.

use crate::{Backend, Document, Error, PagePositions};

/// Scripted behavior of one page.
#[derive(Clone, Debug)]
pub struct StubPage {
    /// Outcome of [`Document::page_text`] for this page.
    pub text: Result<String, Error>,
    /// Outcome of [`Document::page_positions`] for this page. `Ok(None)`
    /// models a backend with no positional API.
    pub positions: Result<Option<PagePositions>, Error>,
}

impl Default for StubPage {
    fn default() -> Self {
        StubPage {
            text: Ok(String::new()),
            positions: Ok(None),
        }
    }
}

/// Scriptable [`Backend`]. Pages start empty (`Ok("")` text, `Ok(None)`
/// positions); script per-page behavior by mutating [`StubBackend::pages`]
/// before opening.
#[derive(Debug)]
pub struct StubBackend {
    /// Copied into each opened document.
    pub pages: Vec<StubPage>,
    /// When set, [`Backend::open`] fails with this error instead of opening.
    pub open_error: Option<Error>,
}

impl StubBackend {
    /// Backend with `page_count` empty pages; `open` succeeds.
    pub fn new(page_count: usize) -> Self {
        StubBackend {
            pages: vec![StubPage::default(); page_count],
            open_error: None,
        }
    }
}

impl Backend for StubBackend {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn open(&self, _path: &str, _password: Option<&str>) -> Result<Box<dyn Document>, Error> {
        match &self.open_error {
            Some(e) => Err(e.clone()),
            None => Ok(Box::new(StubDocument {
                pages: self.pages.clone(),
            })),
        }
    }
}

/// Document opened by [`StubBackend`]. Holds a copy of the scripted pages.
pub struct StubDocument {
    pages: Vec<StubPage>,
}

impl StubDocument {
    fn page(&self, page: usize) -> Result<&StubPage, Error> {
        self.pages.get(page).ok_or_else(|| {
            Error::Other(format!(
                "page {page} out of range ({} pages)",
                self.pages.len()
            ))
        })
    }
}

impl Document for StubDocument {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        self.page(page)?.text.clone()
    }

    fn page_positions(&self, page: usize) -> Result<Option<PagePositions>, Error> {
        self.page(page)?.positions.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, Line, Word};

    fn error_kinds() -> Vec<Error> {
        vec![
            Error::NotFound("no such file".into()),
            Error::PermissionDenied("not readable".into()),
            Error::Encrypted("locked".into()),
            Error::WrongPassword("bad pass".into()),
            Error::NoTextLayer,
            Error::Malformed("not a pdf".into()),
            Error::Unsupported("no backend".into()),
            Error::Other("something else".into()),
        ]
    }

    #[test]
    fn name_is_stub() {
        assert_eq!(StubBackend::new(0).name(), "stub");
    }

    #[test]
    fn open_returns_document_with_scripted_page_count() {
        let doc = StubBackend::new(3).open("x.pdf", None).unwrap();
        assert_eq!(doc.page_count(), 3);
    }

    #[test]
    fn page_count_is_cached_and_infallible() {
        let doc = StubBackend::new(2).open("x.pdf", None).unwrap();
        assert_eq!(doc.page_count(), 2);
        assert_eq!(doc.page_count(), 2);
    }

    #[test]
    fn default_pages_extract_empty_text() {
        let doc = StubBackend::new(2).open("x.pdf", None).unwrap();
        assert_eq!(doc.page_text(0).unwrap(), "");
        assert_eq!(doc.page_text(1).unwrap(), "");
    }

    #[test]
    fn page_text_is_lazy_and_per_page() {
        let mut backend = StubBackend::new(2);
        backend.pages[0].text = Ok("alpha".into());
        backend.pages[1].text = Ok("beta".into());
        let doc = backend.open("x.pdf", None).unwrap();
        assert_eq!(doc.page_text(1).unwrap(), "beta");
        assert_eq!(doc.page_text(0).unwrap(), "alpha");
    }

    #[test]
    fn page_out_of_range_is_an_error() {
        let doc = StubBackend::new(1).open("x.pdf", None).unwrap();
        let err = doc.page_text(1).unwrap_err();
        assert!(matches!(err, Error::Other(_)));
        assert!(matches!(doc.page_positions(1), Err(Error::Other(_))));
    }

    #[test]
    fn scripted_text_errors_flow_verbatim() {
        for kind in error_kinds() {
            let mut backend = StubBackend::new(1);
            backend.pages[0].text = Err(kind.clone());
            let doc = backend.open("x.pdf", None).unwrap();
            assert_eq!(doc.page_text(0).unwrap_err(), kind);
        }
    }

    #[test]
    fn default_positions_model_no_positional_api() {
        let doc = StubBackend::new(1).open("x.pdf", None).unwrap();
        assert_eq!(doc.page_positions(0).unwrap(), None);
    }

    #[test]
    fn scripted_positions_round_trip() {
        let positions = PagePositions {
            blocks: vec![Block {
                lines: vec![Line {
                    words: vec![Word {
                        text: "hello".into(),
                        x: 1.0,
                        y: 2.5,
                        font_size: 11.0,
                    }],
                }],
            }],
        };
        let mut backend = StubBackend::new(1);
        backend.pages[0].positions = Ok(Some(positions.clone()));
        let doc = backend.open("x.pdf", None).unwrap();
        let got = doc.page_positions(0).unwrap().unwrap();
        assert_eq!(got.blocks.len(), 1);
        assert_eq!(got.blocks[0].lines.len(), 1);
        assert_eq!(got.blocks[0].lines[0].words.len(), 1);
        let word = &got.blocks[0].lines[0].words[0];
        assert_eq!(word.text, "hello");
        assert_eq!(word.x, 1.0);
        assert_eq!(word.y, 2.5);
        assert_eq!(word.font_size, 11.0);
    }

    #[test]
    fn scripted_positions_errors_flow_verbatim() {
        let mut backend = StubBackend::new(1);
        backend.pages[0].positions = Err(Error::Malformed("bad geometry".into()));
        let doc = backend.open("x.pdf", None).unwrap();
        let err = doc.page_positions(0).unwrap_err();
        assert_eq!(err, Error::Malformed("bad geometry".into()));
    }

    #[test]
    fn scripted_open_error_flows_verbatim() {
        for kind in error_kinds() {
            let backend = StubBackend {
                pages: vec![StubPage::default()],
                open_error: Some(kind.clone()),
            };
            let got = match backend.open("x.pdf", None) {
                Err(e) => e,
                Ok(_) => panic!("expected open to fail"),
            };
            assert_eq!(got, kind);
        }
    }
}
