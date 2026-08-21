// SPDX-License-Identifier: AGPL-3.0

//! Off-thread page rendering.
//!
//! One worker thread owns a shared [`Document`] and talks to the UI thread over
//! two mpsc channels. Requests arrive in priority order (current page >
//! adjacent pages > ±2 prefetch); the worker drains everything queued at that
//! moment, keeps only the newest request per page via [`coalesce`], and renders
//! sequentially. Dropping the [`Pipeline`] closes the request channel and the
//! worker exits after its current page.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use candi_pdf::{Document, PageImage};

/// A page render job. `scale_q` keys caches; `scale` drives the backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderRequest {
    pub page: usize,
    pub scale_q: u16,
    /// Exact pixels-per-point used for rendering.
    pub scale: f32,
}

impl RenderRequest {
    pub fn key(self) -> crate::render::cache::CacheKey {
        crate::render::cache::CacheKey {
            page: self.page,
            scale_q: self.scale_q,
        }
    }
}

/// Outcome of one queued render.
#[derive(Debug)]
pub enum RenderResult {
    Ready {
        request: RenderRequest,
        image: PageImage,
    },
    Failed {
        request: RenderRequest,
        error: String,
    },
}

/// Drop queued requests so only the newest per **page** survives — a stale
/// zoom's queued render must not survive a newer one — ordered by the arrival
/// position of that newest request. Submitting in priority order therefore
/// keeps current > adjacent > prefetch ordering.
pub fn coalesce(requests: &[RenderRequest]) -> Vec<RenderRequest> {
    let mut latest_index = std::collections::HashMap::new();
    for (i, req) in requests.iter().enumerate() {
        latest_index.insert(req.page, i);
    }
    let mut positions: Vec<usize> = latest_index.values().copied().collect();
    positions.sort_unstable();
    positions.into_iter().map(|i| requests[i]).collect()
}

/// Render one request, converting a panic in the backend into `Failed` so a
/// panicking page cannot kill the worker thread and stall every pending job.
fn render_isolated(document: &dyn Document, req: RenderRequest) -> RenderResult {
    match catch_unwind(AssertUnwindSafe(|| {
        document.render_page(req.page, req.scale)
    })) {
        Ok(Ok(image)) => RenderResult::Ready {
            request: req,
            image,
        },
        Ok(Err(err)) => RenderResult::Failed {
            request: req,
            error: err.to_string(),
        },
        Err(payload) => RenderResult::Failed {
            request: req,
            error: panic_message(&payload),
        },
    }
}

/// Best-effort payload extraction; Rust panics carry `String` or `&str`.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    let detail = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()));
    match detail {
        Some(detail) => format!("renderer panicked: {detail}"),
        None => "renderer panicked".into(),
    }
}

/// Worker-side render pipeline. Cheap to drop: the thread outlives the last
/// queued page at most.
pub struct Pipeline {
    tx: Sender<RenderRequest>,
    rx: Receiver<RenderResult>,
}

impl Pipeline {
    /// Spawn the worker sharing `document`. Thread creation is expected to
    /// work; failure aborts because a reader without its renderer is useless.
    pub fn spawn(document: Arc<dyn Document>) -> Pipeline {
        let (tx, worker_rx) = channel::<RenderRequest>();
        let (result_tx, rx) = channel();
        thread::Builder::new()
            .name("candi-render".into())
            .spawn(move || {
                while let Ok(first) = worker_rx.recv() {
                    let mut batch = vec![first];
                    while let Ok(req) = worker_rx.try_recv() {
                        batch.push(req);
                    }
                    for req in coalesce(&batch) {
                        let result = render_isolated(&*document, req);
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                }
            })
            .expect("spawn candi-render worker thread");
        Pipeline { tx, rx }
    }

    /// Queue renders for execution on the worker. Returns `false` when the
    /// worker has stopped (its receiver is gone), meaning nothing will ever
    /// process these or future requests.
    pub fn submit(&self, requests: &[RenderRequest]) -> bool {
        let mut queued = true;
        for req in requests {
            if self.tx.send(*req).is_err() {
                queued = false;
            }
        }
        queued
    }

    /// Take all completed results without blocking.
    pub fn poll(&self) -> Vec<RenderResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.rx.try_recv() {
            results.push(result);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::time::Instant;

    use candi_pdf::stub::StubBackend;
    use candi_pdf::{Backend, Error, PagePositions};

    fn req(page: usize, scale_q: u16) -> RenderRequest {
        RenderRequest {
            page,
            scale_q,
            scale: scale_q as f32 / 100.0,
        }
    }

    /// A document whose renders always panic; the worker must survive it.
    struct PanickingDoc;

    impl Document for PanickingDoc {
        fn page_count(&self) -> usize {
            2
        }
        fn page_text(&self, _page: usize) -> Result<String, Error> {
            Ok(String::new())
        }
        fn page_positions(&self, _page: usize) -> Result<Option<PagePositions>, Error> {
            Ok(None)
        }
        fn page_size(&self, _page: usize) -> Result<(f32, f32), Error> {
            Ok((612.0, 792.0))
        }
        fn render_page(&self, _page: usize, _scale: f32) -> Result<PageImage, Error> {
            panic!("render boom");
        }
        fn outline(&self) -> Result<Vec<candi_pdf::TocItem>, Error> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn coalesce_keeps_newest_per_key_in_last_seen_order() {
        let batch = vec![
            req(0, 100),
            req(1, 100),
            req(0, 100),
            req(2, 100),
            req(1, 125),
        ];
        assert_eq!(
            coalesce(&batch),
            vec![req(0, 100), req(2, 100), req(1, 125)]
        );
    }

    #[test]
    fn coalesce_preserves_unique_batches_and_handles_empty() {
        let batch = vec![req(2, 100), req(0, 100)];
        assert_eq!(coalesce(&batch), batch);
        assert!(coalesce(&[]).is_empty());
    }

    #[test]
    fn panicking_render_yields_failed_and_worker_survives() {
        let doc: Arc<dyn Document> = Arc::new(PanickingDoc);
        let pipeline = Pipeline::spawn(doc);

        let deadline = Instant::now() + Duration::from_secs(5);
        for page in [0, 1] {
            assert!(pipeline.submit(&[req(page, 100)]));
            loop {
                let polled = pipeline.poll();
                if !polled.is_empty() {
                    match &polled[..] {
                        [RenderResult::Failed { request, error }] => {
                            assert_eq!(request.page, page);
                            assert!(error.contains("panicked"), "{error}");
                        }
                        other => panic!("unexpected results: {other:?}"),
                    }
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "worker produced no result in time"
                );
                thread::sleep(Duration::from_millis(5));
            }
        }
    }

    #[test]
    fn worker_renders_submitted_pages() {
        let doc: Arc<dyn Document> = Arc::from(StubBackend::new(3).open("x.pdf", None).unwrap());
        let pipeline = Pipeline::spawn(doc);
        pipeline.submit(&[req(0, 50)]);
        let deadline = Instant::now() + Duration::from_secs(5);
        let results = loop {
            let polled = pipeline.poll();
            if !polled.is_empty() {
                break polled;
            }
            assert!(
                Instant::now() < deadline,
                "worker produced no result in time"
            );
            thread::sleep(Duration::from_millis(5));
        };
        match &results[..] {
            [RenderResult::Ready { request, image }] => {
                assert_eq!(request.page, 0);
                // Stub pages are 612x792 pt; scale 0.5 → 306x396 px.
                assert_eq!((image.width, image.height), (306, 396));
            }
            [RenderResult::Failed { error, .. }, ..] => panic!("render failed: {error}"),
            other => panic!("unexpected results: {other:?}"),
        }
    }
}
