// SPDX-License-Identifier: AGPL-3.0

//! Open-time text-layer sampling (FR-003).
//!
//! Samples the first few pages only — never scans the whole document.

use crate::{Document, Error};

const ZERO_PAGE_MALFORMED: &str = "truncated or empty document";

/// Number of leading pages to sample when `page_count` is at least this large.
pub const SAMPLE_PAGE_COUNT: usize = 3;

/// After open, sample the first pages for extractable text.
///
/// Empty individual pages are fine (`Ok("")` per architecture). If every sampled
/// page is empty, the document is treated as image-only/scanned.
pub fn reject_if_no_text_layer(doc: &dyn Document) -> Result<(), Error> {
    let page_count = doc.page_count();
    if page_count == 0 {
        return Err(Error::Malformed(ZERO_PAGE_MALFORMED.into()));
    }

    let sample_count = page_count.min(SAMPLE_PAGE_COUNT);
    for page in 0..sample_count {
        if !doc.page_text(page)?.trim().is_empty() {
            return Ok(());
        }
    }

    Err(Error::NoTextLayer)
}
