// SPDX-License-Identifier: AGPL-3.0

//! Off-thread full-document search.
//!
//! One worker thread drives [`SearchSession::step`] page by page, sending
//! each finished page's hits over an mpsc channel; the UI drains the channel
//! per frame, so large documents never block input. Dropping the job or
//! setting `cancel` stops the scan; the thread exits after its current page
//! at the latest.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread;

use candi_core::{SearchSession, normalize_reader_text};
use candi_pdf::Document;

use crate::render::pipeline::panic_message;
use crate::sidebar::{SearchHit, extract_snippet};

/// One running full-document search. Batches of hits arrive page by page in
/// document order; [`SearchJob::poll`] drains everything finished so far.
/// A panic in the scan is isolated like the render pipeline's: it becomes a
/// terminal `Err` batch instead of silently ending the scan mid-document.
pub(crate) struct SearchJob {
    rx: Receiver<Result<Vec<SearchHit>, String>>,
    /// Stops the scan promptly; the worker also stops when the UI drops the
    /// job, but only after finishing the page it is scanning.
    pub(crate) cancel: Arc<AtomicBool>,
    /// Whether the UI already jumped to this job's first hit.
    pub(crate) jumped: bool,
}

impl SearchJob {
    /// Spawn a scan over the whole document.
    pub(crate) fn spawn(document: Arc<dyn Document>, query: String) -> SearchJob {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        thread::Builder::new()
            .name("candi-search".into())
            .spawn(move || {
                let scan = catch_unwind(AssertUnwindSafe(|| {
                    run_scan(Arc::clone(&document), query, tx.clone(), flag)
                }));
                if let Err(payload) = scan {
                    let _ = tx.send(Err(panic_message(&payload, "search")));
                }
            })
            .expect("spawn candi-search worker thread");
        SearchJob {
            rx,
            cancel,
            jumped: false,
        }
    }

    /// Take every finished batch plus whether the scan is over. Each batch
    /// holds one page's hits in document order; an `Err` is the scan's
    /// terminal backend failure.
    pub(crate) fn poll(&self) -> (Vec<Result<Vec<SearchHit>, String>>, bool) {
        let mut batches = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(batch) => batches.push(batch),
                Err(TryRecvError::Empty) => return (batches, false),
                Err(TryRecvError::Disconnected) => return (batches, true),
            }
        }
    }
}

/// Scan page by page, shipping each page's hits as it completes. Pages
/// without matches send nothing.
fn run_scan(
    document: Arc<dyn Document>,
    query: String,
    tx: Sender<Result<Vec<SearchHit>, String>>,
    cancel: Arc<AtomicBool>,
) {
    let needle_len = query.to_lowercase().len();
    let mut session = SearchSession::new(document.as_ref(), query, 0);
    let mut emitted = 0usize;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let complete = match session.step() {
            Ok(complete) => complete,
            Err(err) => {
                let _ = tx.send(Err(err.to_string()));
                return;
            }
        };
        let results = session.results();
        if results.len() != emitted {
            // `step` scans exactly one page, so every new hit is on that
            // page — including the step that completed the scan.
            let page = results[emitted].0;
            let fresh = &results[emitted..];
            emitted = results.len();
            let hits = match document.page_text(page) {
                Ok(text) => {
                    let text = normalize_reader_text(&text).to_lowercase();
                    fresh
                        .iter()
                        .map(|&(_, offset)| SearchHit {
                            page,
                            snippet: extract_snippet(&text, offset, needle_len),
                        })
                        .collect()
                }
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                    return;
                }
            };
            if tx.send(Ok(hits)).is_err() {
                return;
            }
        }
        if complete {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use candi_pdf::{Error, PagePositions};

    /// A document with fixed page texts; the first fetch of `gate.0`
    /// blocks until the test releases it, making scan progress
    /// deterministic while later fetches pass through.
    struct FakeDoc {
        pages: Vec<&'static str>,
        gate: Option<Mutex<(usize, Option<Receiver<()>>)>>,
    }

    impl FakeDoc {
        fn doc(pages: Vec<&'static str>) -> Arc<dyn Document> {
            Arc::new(Self { pages, gate: None })
        }

        fn gated(pages: Vec<&'static str>, block_page: usize) -> (Arc<dyn Document>, Sender<()>) {
            let (tx, rx) = channel();
            (
                Arc::new(Self {
                    pages,
                    gate: Some(Mutex::new((block_page, Some(rx)))),
                }),
                tx,
            )
        }
    }

    impl Document for FakeDoc {
        fn page_count(&self) -> usize {
            self.pages.len()
        }
        fn page_text(&self, page: usize) -> Result<String, Error> {
            if let Some(gate) = &self.gate {
                let mut guard = gate.lock().unwrap();
                if guard.0 == page
                    && let Some(rx) = guard.1.take()
                {
                    let _ = rx.recv();
                }
            }
            Ok(self.pages[page].to_owned())
        }
        fn page_positions(&self, _page: usize) -> Result<Option<PagePositions>, Error> {
            Ok(None)
        }
        fn page_size(&self, _page: usize) -> Result<(f32, f32), Error> {
            Ok((612.0, 792.0))
        }
        fn render_page(&self, _page: usize, _scale: f32) -> Result<candi_pdf::PageImage, Error> {
            Err(Error::Other("not rendered".into()))
        }
        fn outline(&self) -> Result<Vec<candi_pdf::TocItem>, Error> {
            Ok(Vec::new())
        }
        fn search_page(&self, _page: usize, _needle: &str) -> Result<Vec<[f32; 4]>, Error> {
            Ok(Vec::new())
        }
    }

    /// Wait for exactly one batch and return it.
    fn next_batch(job: &mut SearchJob) -> Result<Vec<SearchHit>, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let (batches, done) = job.poll();
            match batches.first() {
                Some(batch) => return batch.clone(),
                None if done => panic!("scan ended without the expected batch"),
                None => {}
            }
            assert!(Instant::now() < deadline, "no batch arrived in time");
            thread::sleep(Duration::from_millis(2));
        }
    }

    /// A document whose text extraction panics on one page mid-scan.
    struct PanickingPageDoc {
        pages: Vec<&'static str>,
        panic_page: usize,
    }

    impl Document for PanickingPageDoc {
        fn page_count(&self) -> usize {
            self.pages.len()
        }
        fn page_text(&self, page: usize) -> Result<String, Error> {
            if page == self.panic_page {
                panic!("text boom");
            }
            Ok(self.pages[page].to_owned())
        }
        fn page_positions(&self, _page: usize) -> Result<Option<PagePositions>, Error> {
            Ok(None)
        }
        fn page_size(&self, _page: usize) -> Result<(f32, f32), Error> {
            Ok((612.0, 792.0))
        }
        fn render_page(&self, _page: usize, _scale: f32) -> Result<candi_pdf::PageImage, Error> {
            Err(Error::Other("not rendered".into()))
        }
        fn outline(&self) -> Result<Vec<candi_pdf::TocItem>, Error> {
            Ok(Vec::new())
        }
        fn search_page(&self, _page: usize, _needle: &str) -> Result<Vec<[f32; 4]>, Error> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn panicked_scan_ends_in_a_terminal_error_not_partial_success() {
        let doc: Arc<dyn Document> = Arc::new(PanickingPageDoc {
            pages: vec!["foo a", "foo b", "foo c"],
            panic_page: 1,
        });
        let mut job = SearchJob::spawn(doc, "foo".into());
        let all = drain(&mut job);
        assert!(
            all[..all.len() - 1].iter().all(|batch| batch.is_ok()),
            "hits that landed before the panic still stream: {all:?}"
        );
        match all.last() {
            Some(Err(err)) => assert!(err.contains("panicked"), "{err}"),
            other => panic!("expected a terminal error batch, got {other:?}"),
        }
    }

    /// Drain the job to completion.
    fn drain(job: &mut SearchJob) -> Vec<Result<Vec<SearchHit>, String>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut all = Vec::new();
        loop {
            let (batches, done) = job.poll();
            all.extend(batches);
            if done {
                return all;
            }
            assert!(Instant::now() < deadline, "scan did not terminate in time");
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn hit_pages(batches: &[Result<Vec<SearchHit>, String>]) -> Vec<usize> {
        batches
            .iter()
            .flat_map(|batch| batch.as_ref().unwrap().iter().map(|hit| hit.page))
            .collect()
    }

    #[test]
    fn scan_streams_one_page_at_a_time() {
        let (doc, release) = FakeDoc::gated(vec!["foo bar foo", "nothing here", "foo baz"], 2);
        let mut job = SearchJob::spawn(doc, "foo".into());
        // The first delivery holds only the first page's hits; the worker
        // then stalls on the gated last page.
        let batch = next_batch(&mut job);
        let hits = batch.unwrap();
        assert_eq!(
            hits.iter().map(|hit| hit.page).collect::<Vec<_>>(),
            vec![0, 0],
            "both page-0 matches"
        );
        release.send(()).unwrap();
        let rest = drain(&mut job);
        assert_eq!(
            hit_pages(&rest),
            vec![2],
            "the match-less middle page sends nothing"
        );
    }

    #[test]
    fn cancel_stops_the_scan_after_the_current_page() {
        let (doc, release) = FakeDoc::gated(vec!["foo a", "foo b", "foo c"], 1);
        let mut job = SearchJob::spawn(doc, "foo".into());
        let _ = next_batch(&mut job);
        job.cancel.store(true, Ordering::Relaxed);
        release.send(()).unwrap();
        let rest = drain(&mut job);
        assert_eq!(
            hit_pages(&rest),
            vec![1],
            "the in-flight page still reports; later pages never run"
        );
    }

    #[test]
    fn concurrent_jobs_are_independent_and_terminate() {
        let mut old = SearchJob::spawn(FakeDoc::doc(vec!["foo one", "foo two"]), "foo".into());
        let mut fresh = SearchJob::spawn(FakeDoc::doc(vec!["bar only", "bar again"]), "bar".into());
        let old_hits = drain(&mut old);
        let fresh_hits = drain(&mut fresh);
        assert_eq!(hit_pages(&old_hits), vec![0, 1]);
        assert_eq!(hit_pages(&fresh_hits), vec![0, 1]);
        for batch in &fresh_hits {
            for hit in batch.as_ref().unwrap() {
                assert!(hit.snippet.contains("bar"), "{:?}", hit.snippet);
                assert!(!hit.snippet.contains("foo"), "{:?}", hit.snippet);
            }
        }
    }
}
