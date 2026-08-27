// SPDX-License-Identifier: AGPL-3.0

use std::sync::atomic::{AtomicUsize, Ordering};

use candi_core::{SearchSession, normalize_reader_text};
use candi_pdf::{Document, Error};

struct FakeDoc {
    pages: Vec<&'static str>,
    page_text_calls: AtomicUsize,
}

impl FakeDoc {
    fn new(pages: Vec<&'static str>) -> Self {
        Self {
            pages,
            page_text_calls: AtomicUsize::new(0),
        }
    }

    fn page_text_call_count(&self) -> usize {
        self.page_text_calls.load(Ordering::SeqCst)
    }
}

impl Document for FakeDoc {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        self.page_text_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.pages[page].to_string())
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

    fn search_page(&self, _page: usize, _needle: &str) -> Result<Vec<[f32; 4]>, Error> {
        Ok(Vec::new())
    }
}

struct ErrorOnPageDoc {
    fail_page: usize,
}

impl Document for ErrorOnPageDoc {
    fn page_count(&self) -> usize {
        3
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        if page == self.fail_page {
            return Err(Error::Malformed("boom".into()));
        }
        Ok(String::new())
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

    fn search_page(&self, _page: usize, _needle: &str) -> Result<Vec<[f32; 4]>, Error> {
        Ok(Vec::new())
    }
}

static ERROR_DOC_CALLS: AtomicUsize = AtomicUsize::new(0);

struct CountingErrorDoc;

impl Document for CountingErrorDoc {
    fn page_count(&self) -> usize {
        1
    }

    fn page_text(&self, _page: usize) -> Result<String, Error> {
        ERROR_DOC_CALLS.fetch_add(1, Ordering::SeqCst);
        Err(Error::Malformed("fail".into()))
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

    fn search_page(&self, _page: usize, _needle: &str) -> Result<Vec<[f32; 4]>, Error> {
        Ok(Vec::new())
    }
}

const PAGES: &[&str] = &["alpha foo", "bar", "foo baz foo"];

#[test]
fn new_does_not_call_page_text() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let _session = SearchSession::new(&doc, "foo", 0);
    assert_eq!(doc.page_text_call_count(), 0);
}

#[test]
fn first_next_is_lazy_and_starts_at_start_page() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);
    let hit = session.next().unwrap().unwrap();
    assert_eq!(hit, (0, 6));
    assert!(doc.page_text_call_count() < doc.page_count());
}

#[test]
fn first_next_from_later_start_page() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 2);
    let hit = session.next().unwrap().unwrap();
    assert_eq!(hit, (2, 0));
    assert_eq!(doc.page_text_call_count(), 1);
}

#[test]
fn multiple_hits_on_one_page() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);

    session.next().unwrap().unwrap();
    session.prev().unwrap().unwrap();

    assert!(session.results().contains(&(2, 0)));
    assert!(session.results().contains(&(2, 8)));
}

#[test]
fn next_wraps_after_full_scan() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);

    let first = session.next().unwrap().unwrap();
    session.prev().unwrap().unwrap();
    let last = session.current().unwrap();
    assert_ne!(first, last);

    let wrapped = session.next().unwrap().unwrap();
    assert_eq!(wrapped, first);
}

#[test]
fn prev_wraps_after_full_scan() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);

    session.next().unwrap().unwrap();
    session.prev().unwrap().unwrap();
    let last = session.current().unwrap();
    session.next().unwrap().unwrap();

    let wrapped = session.prev().unwrap().unwrap();
    assert_eq!(wrapped, last);
}

#[test]
fn case_insensitive_match() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "Foo", 0);
    let hit = session.next().unwrap().unwrap();
    assert_eq!(hit, (0, 6));
}

#[test]
fn step_scans_exactly_one_page_per_call() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);

    assert!(!session.step().unwrap(), "page 0 is not the last page");
    let after_first = doc.page_text_call_count();
    assert_eq!(after_first, 1, "one step = one page_text call");
    assert!(session.results().contains(&(0, 6)));

    assert!(!session.step().unwrap());
    assert_eq!(doc.page_text_call_count(), 2);
    assert_eq!(
        session.results(),
        &[(0, 6)],
        "page 1 has no matches; earlier results persist"
    );

    assert!(session.step().unwrap(), "the scan ends at the last page");
    assert_eq!(doc.page_text_call_count(), 3);
    assert!(session.results().contains(&(2, 0)));
    assert!(session.results().contains(&(2, 8)));

    assert!(session.step().unwrap(), "stepping a complete scan is idle");
    assert_eq!(doc.page_text_call_count(), 3);
}

#[test]
fn empty_query_no_page_text_calls() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "", 0);
    assert!(session.next().unwrap().is_none());
    assert!(session.prev().unwrap().is_none());
    assert!(session.current().is_none());
    assert!(session.results().is_empty());
    assert_eq!(doc.page_text_call_count(), 0);
}

#[test]
fn empty_document() {
    let doc = FakeDoc::new(vec![]);
    let mut session = SearchSession::new(&doc, "foo", 0);
    assert!(session.next().unwrap().is_none());
    assert!(session.prev().unwrap().is_none());
    assert!(session.current().is_none());
    assert!(session.results().is_empty());
    assert_eq!(doc.page_text_call_count(), 0);
}

#[test]
fn page_text_error_propagates() {
    ERROR_DOC_CALLS.store(0, Ordering::SeqCst);
    let doc = CountingErrorDoc;
    let mut session = SearchSession::new(&doc, "x", 0);
    let err = session.next().unwrap_err();
    assert!(matches!(err, Error::Malformed { .. }));
    assert_eq!(ERROR_DOC_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn page_text_error_on_later_page() {
    let doc = ErrorOnPageDoc { fail_page: 1 };
    let mut session = SearchSession::new(&doc, "foo", 0);
    assert!(session.next().is_err());
}

#[test]
fn results_hold_offsets_not_text() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);
    session.next().unwrap().unwrap();
    for (page, offset) in session.results() {
        assert!(*page < doc.page_count());
        let text = normalize_reader_text(&doc.page_text(*page).unwrap()).to_lowercase();
        assert!(text[*offset..].starts_with("foo"));
    }
}

#[test]
fn query_accessor_returns_original_case_insensitive_needle() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let session = SearchSession::new(&doc, "Foo", 0);
    assert_eq!(session.query(), "foo");
}

#[test]
fn start_page_clamped_like_view_state() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 99);
    let hit = session.next().unwrap().unwrap();
    assert_eq!(hit, (2, 0));
    assert_eq!(doc.page_text_call_count(), 1);
}

#[test]
fn start_page_full_scan_does_not_rescan_start_page() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 2);

    session.next().unwrap().unwrap();
    session.prev().unwrap().unwrap();

    assert_eq!(doc.page_text_call_count(), doc.page_count());
    let expected = [(0_usize, 6_usize), (2, 0), (2, 8)];
    assert_eq!(session.results().len(), expected.len());
    for hit in expected {
        assert!(session.results().contains(&hit));
    }
    assert_eq!(
        session
            .results()
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        expected.len()
    );
}

#[test]
fn non_ascii_page_text_does_not_panic_and_finds_hits() {
    let doc = FakeDoc::new(vec!["café latte — résumé"]);
    let mut session = SearchSession::new(&doc, "latte", 0);
    assert_eq!(session.next().unwrap().unwrap(), (0, 6));
    assert_eq!(session.results().len(), 1);

    let mut session = SearchSession::new(&doc, "résumé", 0);
    assert_eq!(session.next().unwrap().unwrap(), (0, 14));
}

#[test]
fn search_matches_normalized_ligatures() {
    struct LigatureDoc {
        page: String,
    }

    impl Document for LigatureDoc {
        fn page_count(&self) -> usize {
            1
        }

        fn page_text(&self, page: usize) -> Result<String, Error> {
            if page == 0 {
                Ok(self.page.clone())
            } else {
                Err(Error::Malformed("bad page".into()))
            }
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

        fn search_page(&self, _page: usize, _needle: &str) -> Result<Vec<[f32; 4]>, Error> {
            Ok(Vec::new())
        }
    }

    let doc = LigatureDoc {
        page: format!("{}nger", '\u{fb01}'),
    };
    let mut session = SearchSession::new(&doc, "finger", 0);
    let hit = session.next().unwrap().unwrap();
    assert_eq!(hit, (0, 0));
}

#[test]
fn prev_finishes_scan_before_wrap_from_first() {
    let doc = FakeDoc::new(PAGES.to_vec());
    let mut session = SearchSession::new(&doc, "foo", 0);
    session.next().unwrap().unwrap();
    let prev = session.prev().unwrap().unwrap();
    assert_eq!(session.results().len(), 3);
    assert_eq!(prev, (2, 8));
}
