// SPDX-License-Identifier: AGPL-3.0

//! Document-backend trait layer for candi.
//!
//! The trait surface is pinned in docs/architecture.md §candi-pdf: error kinds,
//! `Document`/`Backend` signatures, and the positions types are copied
//! verbatim from there. Backends (MuPDF, PDFium) implement these traits in
//! later slices; the factory and the test stub live here.

#[cfg(any(feature = "mupdf-backend", feature = "pdfium-backend"))]
mod backend;
mod error;
mod factory;
pub mod stub;
#[cfg(any(feature = "mupdf-backend", feature = "pdfium-backend"))]
mod textlayer;

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

/// A rendered page: RGBA8, row-major, top-left origin, fully opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Exactly `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

impl PageImage {
    /// Builds from a tightly packed RGBA buffer. Errors when the buffer does
    /// not match the dimensions, so every [`PageImage`] honors its length
    /// invariant.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, Error> {
        let expected = width as usize * height as usize * 4;
        if rgba.len() != expected {
            return Err(Error::Other(format!(
                "image buffer holds {} bytes, expected {expected} for {width}x{height}",
                rgba.len()
            )));
        }
        Ok(PageImage {
            width,
            height,
            rgba,
        })
    }

    /// Builds from tightly packed RGB samples, expanding to RGBA in one pass
    /// with full opacity. Errors when the buffer does not match the dimensions.
    pub fn from_rgb(width: u32, height: u32, rgb: &[u8]) -> Result<Self, Error> {
        let expected = width as usize * height as usize * 3;
        if rgb.len() != expected {
            return Err(Error::Other(format!(
                "RGB buffer holds {} bytes, expected {expected} for {width}x{height}",
                rgb.len()
            )));
        }
        let mut rgba = Vec::with_capacity(expected / 3 * 4);
        for pixel in rgb.chunks_exact(3) {
            rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], u8::MAX]);
        }
        Ok(PageImage {
            width,
            height,
            rgba,
        })
    }
}

/// One table-of-contents entry; `page` is 1-based.
#[derive(Debug, Clone, PartialEq)]
pub struct TocItem {
    pub title: String,
    pub page: usize,
    /// Vertical landing point within the page, in points measured from the
    /// page's top edge (the same top-left point space as `search_page`
    /// rects), when the destination carries one; `None` for page-only or
    /// fit-style destinations.
    pub dest_top: Option<f32>,
    pub children: Vec<TocItem>,
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
    /// Page dimensions in PDF points (1/72 inch), as `(width, height)`.
    fn page_size(&self, page: usize) -> Result<(f32, f32), Error>;
    /// Render one page at `scale` pixels per point into an RGBA image.
    fn render_page(&self, page: usize, scale: f32) -> Result<PageImage, Error>;
    /// Document outline (table of contents) as a tree of 1-based pages;
    /// empty when the document has none. Entries whose destination is
    /// external or unresolvable are dropped, so a present-but-unusable
    /// outline can come back empty.
    fn outline(&self) -> Result<Vec<TocItem>, Error>;
    /// Rectangles of every case-insensitive occurrence of `needle` on one
    /// page, one entry per highlight box (a match spanning lines yields one
    /// entry per line): `[x0, y0, x1, y1]` in PDF points (1/72 inch),
    /// TOP-LEFT origin with y increasing downward. An empty needle returns
    /// no rects.
    fn search_page(&self, page: usize, needle: &str) -> Result<Vec<[f32; 4]>, Error>;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rgb_expands_to_opaque_rgba_in_one_pass() {
        let image = PageImage::from_rgb(2, 1, &[1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn from_rgb_rejects_buffer_that_does_not_match_dimensions() {
        let err = PageImage::from_rgb(2, 2, &[0; 11]).unwrap_err();
        assert!(matches!(err, Error::Other(_)));
    }

    #[test]
    fn from_rgba_rejects_buffer_that_does_not_match_dimensions() {
        let err = PageImage::from_rgba(2, 2, vec![0; 15]).unwrap_err();
        assert!(matches!(err, Error::Other(_)));

        let ok = PageImage::from_rgba(2, 2, vec![7; 16]).unwrap();
        assert_eq!(ok.rgba.len(), 16);
    }
}
