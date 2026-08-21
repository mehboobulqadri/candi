// SPDX-License-Identifier: AGPL-3.0

//! MuPDF engine behind the `mupdf-backend` feature.

use std::io;
use std::sync::Mutex;

use mupdf::{
    Colorspace, Document as MupdfDocument, Error as MupdfError, Matrix, TextExtractOptions,
    TextPageFlags,
};

use crate::{Backend, Block, Document, Error, Line, PageImage, PagePositions, TocItem, Word};

const FZ_ERROR_FORMAT: i32 = 7;
const FZ_ERROR_SYNTAX: i32 = 8;
const FZ_ERROR_UNSUPPORTED: i32 = 6;

/// Returned when MuPDF opens a file but reports zero pages (silent-open class).
pub(crate) const ZERO_PAGE_MALFORMED: &str = "truncated or empty document";

/// MuPDF-backed document engine.
#[derive(Debug, Default)]
pub struct MupdfBackend;

impl Backend for MupdfBackend {
    fn name(&self) -> &'static str {
        "mupdf"
    }

    fn open(&self, path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error> {
        preflight_path(path)?;
        let mut doc = MupdfDocument::open(path).map_err(map_mupdf_error)?;

        let page_count = doc
            .page_count()
            .map_err(map_mupdf_error)?
            .try_into()
            .map_err(|_| Error::Other("page count out of range".into()))?;

        if page_count == 0 {
            return Err(Error::Malformed(ZERO_PAGE_MALFORMED.into()));
        }

        if doc.needs_password().map_err(map_mupdf_error)? {
            match password {
                None => {
                    return Err(Error::Encrypted("document requires a password".into()));
                }
                Some(pass) => {
                    let ok = doc.authenticate(pass).map_err(map_mupdf_error)?;
                    if !ok {
                        return Err(Error::WrongPassword("password rejected".into()));
                    }
                }
            }
        }

        let document = Box::new(MupdfPdfDocument {
            inner: Mutex::new(MupdfInner { doc }),
            page_count,
        });
        crate::textlayer::reject_if_no_text_layer(document.as_ref())?;
        Ok(document)
    }
}

struct MupdfInner {
    doc: MupdfDocument,
}

// The mupdf crate resolves work through a thread-local `fz_context` (`Context::get()` /
// `context()`); we hold the opened `fz_document` and serialize access through the mutex.
// MuPDF keeps the document alive for loaded pages — see the crate's page-survives-
// document-drop test. The decoded-resource store lives in that TLS context; dropping a
// `Page` does not empty MuPDF's default ~256 MiB store ceiling.
unsafe impl Send for MupdfInner {}
unsafe impl Sync for MupdfInner {}

struct MupdfPdfDocument {
    inner: Mutex<MupdfInner>,
    page_count: usize,
}

impl Document for MupdfPdfDocument {
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().expect("mupdf document mutex poisoned");
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        mupdf_page
            .text(TextExtractOptions::default())
            .map_err(map_mupdf_error)
    }

    fn page_positions(&self, page: usize) -> Result<Option<PagePositions>, Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().expect("mupdf document mutex poisoned");
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        let text_page = mupdf_page
            .to_text_page(TextPageFlags::empty())
            .map_err(map_mupdf_error)?;
        Ok(Some(positions_from_text_page(&text_page)))
    }

    fn page_size(&self, page: usize) -> Result<(f32, f32), Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().expect("mupdf document mutex poisoned");
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        let bounds = mupdf_page.bounds().map_err(map_mupdf_error)?;
        Ok((
            (bounds.x1 - bounds.x0).max(0.0),
            (bounds.y1 - bounds.y0).max(0.0),
        ))
    }

    fn render_page(&self, page: usize, scale: f32) -> Result<PageImage, Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().expect("mupdf document mutex poisoned");
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        let pixmap = mupdf_page
            .to_pixmap(
                &Matrix::new_scale(scale, scale),
                &Colorspace::device_rgb(),
                false,
                true,
            )
            .map_err(map_mupdf_error)?;
        PageImage::from_rgb(pixmap.width(), pixmap.height(), pixmap.samples())
    }

    fn outline(&self) -> Result<Vec<TocItem>, Error> {
        let inner = self.inner.lock().expect("mupdf document mutex poisoned");
        let outlines = inner.doc.outlines().map_err(map_mupdf_error)?;
        Ok(toc_from_outlines(&outlines))
    }
}

/// `LinkDestination::page_number` is the 0-based absolute page; entries with
/// no destination (external links, unresolvable URIs) are dropped.
fn toc_from_outlines(outlines: &[mupdf::Outline]) -> Vec<TocItem> {
    outlines
        .iter()
        .filter_map(|outline| {
            let page = outline.dest.as_ref()?.loc.page_number as usize + 1;
            Some(TocItem {
                title: outline.title.clone(),
                page,
                children: toc_from_outlines(&outline.down),
            })
        })
        .collect()
}

fn page_index(page: usize, page_count: usize) -> Result<i32, Error> {
    if page >= page_count {
        return Err(Error::Other(format!(
            "page {page} out of range ({page_count} pages)"
        )));
    }
    i32::try_from(page).map_err(|_| Error::Other(format!("page index {page} out of range")))
}

fn positions_from_text_page(text_page: &mupdf::TextPage) -> PagePositions {
    let structured = text_page.structured();
    let words = text_page.words();
    let mut blocks = Vec::with_capacity(structured.blocks.len());

    for (block_idx, block) in structured.blocks.iter().enumerate() {
        let mupdf::text_page::TextBlockContent::Text { lines } = &block.content else {
            continue;
        };

        let mut out_lines = Vec::with_capacity(lines.len());
        for (line_idx, line) in lines.iter().enumerate() {
            let line_words: Vec<_> = words
                .iter()
                .filter(|w| w.block == block_idx && w.line == line_idx)
                .collect();

            if line_words.is_empty() && line.text.is_empty() {
                continue;
            }

            let mut out_words = Vec::with_capacity(line_words.len());
            for word in line_words {
                let font_size = (word.bounds.y1 - word.bounds.y0).max(0.0);
                out_words.push(Word {
                    text: word.text.clone(),
                    x: word.bounds.x0,
                    y: word.bounds.y0,
                    font_size,
                });
            }

            if !out_words.is_empty() {
                out_lines.push(Line { words: out_words });
            }
        }

        if !out_lines.is_empty() {
            blocks.push(Block { lines: out_lines });
        }
    }

    PagePositions { blocks }
}

fn preflight_path(path: &str) -> Result<(), Error> {
    match std::fs::metadata(path) {
        Err(err) => return Err(map_io_error(err)),
        Ok(meta) if meta.is_dir() => {
            return Err(Error::NotFound(format!("{path} is a directory")));
        }
        Ok(_) => {}
    }
    if let Err(err) = std::fs::File::open(path) {
        return Err(map_io_error(err));
    }
    Ok(())
}

fn map_mupdf_error(err: MupdfError) -> Error {
    match err {
        MupdfError::Io(io_err) => map_io_error(io_err),
        MupdfError::InvalidPdfDocument => Error::Malformed("invalid PDF document".into()),
        MupdfError::MuPdf(mupdf_err) => map_mupdf_code(mupdf_err.code, mupdf_err.message),
        other => Error::Other(other.to_string()),
    }
}

fn map_io_error(err: io::Error) -> Error {
    match err.kind() {
        io::ErrorKind::NotFound => Error::NotFound(err.to_string()),
        io::ErrorKind::PermissionDenied => Error::PermissionDenied(err.to_string()),
        _ => Error::Other(err.to_string()),
    }
}

fn map_mupdf_code(code: i32, message: String) -> Error {
    match code {
        FZ_ERROR_FORMAT | FZ_ERROR_SYNTAX => Error::Malformed(message),
        FZ_ERROR_UNSUPPORTED => Error::Unsupported(message),
        _ => Error::Other(message),
    }
}
