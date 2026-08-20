// SPDX-License-Identifier: AGPL-3.0

//! Document-backend trait layer for candi.
//!
//! The trait surface is pinned in docs/architecture.md §candi-pdf: error kinds,
//! `Document`/`Backend` signatures, and the positions types are copied
//! verbatim from there. Backends (MuPDF, PDFium) implement these traits in
//! later slices; the factory and the test stub live here.

#[cfg(feature = "mupdf-backend")]
mod backend;
mod error;
mod factory;
pub mod stub;

pub use error::Error;
pub use factory::{BackendKind, available, open, open_default};

/// Positions of every word on a page, grouped into blocks and lines.
#[derive(Debug, Clone, PartialEq)]
pub struct PagePositions {
    pub blocks: Vec<Block>,
}

/// Contiguous text region ≈ paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub lines: Vec<Line>,
}

/// Words sharing one baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub words: Vec<Word>,
}

/// Whitespace-free run with origin and font size.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
}

/// An opened document. All methods are thread-safe (`Send + Sync`).
///
/// `open()` parses and validates, caches `page_count`, and loads no text;
/// text extraction is lazy and per page.
pub trait Document: Send + Sync {
    /// Number of pages, cached at open; infallible.
    fn page_count(&self) -> usize;
    /// Extract exactly one page's text; empty pages return `Ok("")`.
    fn page_text(&self, page: usize) -> Result<String, Error>;
    /// Word positions for one page; `Ok(None)` only when the backend has no
    /// positional API. Real errors propagate as `Err`.
    fn page_positions(&self, page: usize) -> Result<Option<PagePositions>, Error>;
}

/// A document backend. Implementations must be cheap to construct and hold no
/// per-document state — state belongs to the [`Document`] it opens.
pub trait Backend: Send + Sync {
    /// Stable identifier, e.g. `"mupdf"`.
    fn name(&self) -> &'static str;
    /// Parse and validate the file at `path`, cache its page count, load no
    /// text. A password is only passed when the CLI has one to give.
    fn open(&self, path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error>;
}
