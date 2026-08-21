// SPDX-License-Identifier: AGPL-3.0

//! Scriptable test double implementing the full trait surface.
//!
//! The parity suite (01/04) and core tests (01/06–08) build on this; every
//! behavior a backend can exhibit is scriptable here.

use crate::{Backend, Document, Error, PageImage, PagePositions, TocItem};

/// Every stub page is US Letter, 612x792 PDF points.
const STUB_PAGE_WIDTH: f32 = 612.0;
const STUB_PAGE_HEIGHT: f32 = 792.0;

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

    fn page_size(&self, page: usize) -> Result<(f32, f32), Error> {
        self.page(page)?;
        Ok((STUB_PAGE_WIDTH, STUB_PAGE_HEIGHT))
    }

    /// White background with black horizontal stripes whose period derives
    /// from the page index — a pure function of `(page, scale)`, so repeated
    /// calls are byte-identical and pages stay distinguishable.
    fn render_page(&self, page: usize, scale: f32) -> Result<PageImage, Error> {
        self.page(page)?;
        let width = (STUB_PAGE_WIDTH * scale).round() as usize;
        let height = (STUB_PAGE_HEIGHT * scale).round() as usize;
        let mut rgba = vec![255u8; width * height * 4];
        let period = (page % 8 + 1) * 8;
        for y in (0..height).step_by(period) {
            for pixel in rgba[y * width * 4..(y + 1) * width * 4].chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
            }
        }
        PageImage::from_rgba(width as u32, height as u32, rgba)
    }

    /// Fixed three-entry tree with one nested child on the first entry, so
    /// consumers always have structure to render.
    fn outline(&self) -> Result<Vec<TocItem>, Error> {
        Ok(vec![
            TocItem {
                title: "Part One".into(),
                page: 1,
                children: vec![TocItem {
                    title: "Chapter 1".into(),
                    page: 2,
                    children: Vec::new(),
                }],
            },
            TocItem {
                title: "Part Two".into(),
                page: 3,
                children: Vec::new(),
            },
            TocItem {
                title: "Part Three".into(),
                page: 4,
                children: Vec::new(),
            },
        ])
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

    #[test]
    fn page_size_is_letter_points_for_valid_pages_only() {
        let doc = StubBackend::new(2).open("x.pdf", None).unwrap();
        assert_eq!(doc.page_size(0).unwrap(), (612.0, 792.0));
        assert_eq!(doc.page_size(1).unwrap(), (612.0, 792.0));
        assert!(matches!(doc.page_size(2), Err(Error::Other(_))));
    }

    #[test]
    fn render_matches_scale_and_buffer_invariant() {
        let doc = StubBackend::new(1).open("x.pdf", None).unwrap();
        let one = doc.render_page(0, 1.0).unwrap();
        assert_eq!((one.width, one.height), (612, 792));
        assert_eq!(one.rgba.len(), 612 * 792 * 4);

        let two = doc.render_page(0, 2.0).unwrap();
        assert_eq!((two.width, two.height), (1224, 1584));
        assert_eq!(two.rgba.len(), two.width as usize * two.height as usize * 4);
    }

    #[test]
    fn render_is_byte_identical_across_calls_and_distinct_per_page() {
        let doc = StubBackend::new(3).open("x.pdf", None).unwrap();
        assert_eq!(
            doc.render_page(1, 2.0).unwrap(),
            doc.render_page(1, 2.0).unwrap()
        );
        assert_ne!(
            doc.render_page(0, 2.0).unwrap().rgba,
            doc.render_page(1, 2.0).unwrap().rgba
        );
        assert_ne!(
            doc.render_page(1, 2.0).unwrap().rgba,
            doc.render_page(2, 2.0).unwrap().rgba
        );
    }

    #[test]
    fn render_out_of_range_page_is_an_error() {
        let doc = StubBackend::new(1).open("x.pdf", None).unwrap();
        assert!(matches!(doc.render_page(1, 1.0), Err(Error::Other(_))));
    }

    #[test]
    fn outline_is_fixed_three_entry_tree_with_nested_child() {
        let doc = StubBackend::new(4).open("x.pdf", None).unwrap();
        let toc = doc.outline().unwrap();
        assert_eq!(toc.len(), 3);
        assert_eq!(toc[0].title, "Part One");
        assert_eq!(toc[0].page, 1);
        assert_eq!(toc[0].children.len(), 1);
        assert_eq!(toc[0].children[0].title, "Chapter 1");
        assert_eq!(toc[0].children[0].page, 2);
        assert_eq!(toc[1].title, "Part Two");
        assert_eq!(toc[1].page, 3);
        assert!(toc[1].children.is_empty());
        assert_eq!(toc[2].title, "Part Three");
        assert_eq!(toc[2].page, 4);
        assert!(toc[2].children.is_empty());
    }
}
