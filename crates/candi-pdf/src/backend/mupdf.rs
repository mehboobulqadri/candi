// SPDX-License-Identifier: AGPL-3.0

//! MuPDF engine behind the `mupdf-backend` feature.

use std::io;
use std::sync::{Mutex, PoisonError};

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
    // `.unwrap_or_else(PoisonError::into_inner)` everywhere: a panic in a
    // prior holder poisons the mutex, but the guarded struct holds no
    // intermediate state and mupdf-rs keeps per-op state in the thread-local
    // `fz_context`, so the next per-page op either succeeds or surfaces the
    // breakage through the normal error path. Recovery beats aborting the
    // reader on the main thread.
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        mupdf_page
            .text(TextExtractOptions::default())
            .map_err(map_mupdf_error)
    }

    fn page_positions(&self, page: usize) -> Result<Option<PagePositions>, Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        let text_page = mupdf_page
            .to_text_page(TextPageFlags::empty())
            .map_err(map_mupdf_error)?;
        Ok(Some(positions_from_text_page(&text_page)))
    }

    fn page_size(&self, page: usize) -> Result<(f32, f32), Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        let bounds = mupdf_page.bounds().map_err(map_mupdf_error)?;
        Ok((
            (bounds.x1 - bounds.x0).max(0.0),
            (bounds.y1 - bounds.y0).max(0.0),
        ))
    }

    fn render_page(&self, page: usize, scale: f32) -> Result<PageImage, Error> {
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
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
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let outlines = inner.doc.outlines().map_err(map_mupdf_error)?;
        Ok(toc_from_outlines(&outlines, self.page_count, 0))
    }

    fn search_page(&self, page: usize, needle: &str) -> Result<Vec<[f32; 4]>, Error> {
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let page_no = page_index(page, self.page_count)?;
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let mupdf_page = inner.doc.load_page(page_no).map_err(map_mupdf_error)?;
        let text_page = mupdf_page
            .to_text_page(TextPageFlags::empty())
            .map_err(map_mupdf_error)?;
        // fz_search folds case and returns quads in page space — top-left
        // origin, y-down points, the same space as word bounds above.
        Ok(text_page
            .search(needle)
            .map_err(map_mupdf_error)?
            .iter()
            .map(|quad| {
                let (ul, ur, ll, lr) = (&quad.ul, &quad.ur, &quad.ll, &quad.lr);
                [
                    ul.x.min(ll.x).min(ur.x).min(lr.x),
                    ul.y.min(ur.y).min(ll.y).min(lr.y),
                    ul.x.max(ll.x).max(ur.x).max(lr.x),
                    ul.y.max(ur.y).max(ll.y).max(lr.y),
                ]
            })
            .collect())
    }
}

/// Maximum outline nesting the walker descends into; malformed documents can
/// build pathological trees. The owned `Vec` tree cannot cycle, so — unlike
/// the PDFium pointer walk — the depth cap alone bounds the recursion (and
/// with it the sidebar's transitive flattening).
const MAX_OUTLINE_DEPTH: usize = 64;

/// `LinkDestination::page_number` is the 0-based absolute page; entries with
/// no destination (external links, unresolvable URIs) are dropped. Negative
/// numbers and pages past the end of the document are dropped too, so a
/// damaged sidecar cannot surface `-1` as a huge 1-based page.
fn toc_from_outlines(outlines: &[mupdf::Outline], page_count: usize, depth: usize) -> Vec<TocItem> {
    if depth > MAX_OUTLINE_DEPTH {
        return Vec::new();
    }
    outlines
        .iter()
        .filter_map(|outline| {
            let dest = outline.dest.as_ref()?;
            let number = i64::from(dest.loc.page_number);
            let page = usize::try_from(number).ok()? + 1;
            (page <= page_count).then_some(TocItem {
                title: outline.title.clone(),
                page,
                dest_top: dest_top(&dest.kind),
                children: toc_from_outlines(&outline.down, page_count, depth + 1),
            })
        })
        .collect()
}

/// Vertical landing point of a destination, in MuPDF page space — top-left
/// origin, y-down, the same space as rendered page bounds — so the value is
/// points from the page's top edge. Destinations without a vertical
/// component (`/Fit`, page-only links) yield `None`, as do non-finite tops,
/// which crafted documents can produce and a clamp would otherwise fold
/// into the scroll math.
fn dest_top(kind: &mupdf::DestinationKind) -> Option<f32> {
    let top = match kind {
        mupdf::DestinationKind::XYZ { top: Some(top), .. }
        | mupdf::DestinationKind::FitH { top: Some(top) }
        | mupdf::DestinationKind::FitBH { top: Some(top) } => *top,
        _ => return None,
    };
    top.is_finite().then_some(top)
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
        Ok(meta) => check_path_type(path, meta)?,
    }
    if let Err(err) = std::fs::File::open(path) {
        return Err(map_io_error(err));
    }
    Ok(())
}

/// Reject existing paths that must not reach the `File::open` probe:
/// directories fail with the established NotFound, and non-regular files —
/// named pipes above all, which would block the open until a writer
/// appears — fail outright.
fn check_path_type(path: &str, meta: std::fs::Metadata) -> Result<(), Error> {
    if meta.is_dir() {
        return Err(Error::NotFound(format!("{path} is a directory")));
    }
    if !meta.is_file() {
        return Err(Error::Other(format!("{path} is not a regular file")));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use mupdf::document::Location;
    use mupdf::link::LinkDestination;
    use mupdf::{DestinationKind, Outline};

    fn outline(title: &str, page_number: u32, down: Vec<Outline>) -> Outline {
        Outline {
            title: title.to_owned(),
            uri: None,
            dest: Some(LinkDestination {
                loc: Location {
                    chapter: 0,
                    page_in_chapter: page_number,
                    page_number,
                },
                kind: DestinationKind::XYZ {
                    left: None,
                    top: Some(72.0),
                    zoom: None,
                },
            }),
            down,
        }
    }

    fn tree_depth(items: &[TocItem]) -> usize {
        items
            .iter()
            .map(|item| 1 + tree_depth(&item.children))
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn outline_walker_keeps_every_level_at_or_below_the_cap() {
        // The guard is `depth > MAX`, so depths 0..=MAX render: one node
        // per depth, MAX + 1 levels for a chain of MAX + 1.
        let chain = (0..MAX_OUTLINE_DEPTH)
            .rev()
            .fold(outline("leaf", 1, Vec::new()), |child, i| {
                outline(&format!("n{i}"), 1, vec![child])
            });
        let toc = toc_from_outlines(&[chain], 3, 0);
        assert_eq!(tree_depth(&toc), MAX_OUTLINE_DEPTH + 1);
    }

    #[test]
    fn outline_walker_stops_at_the_cap() {
        let chain = (0..MAX_OUTLINE_DEPTH * 4)
            .rev()
            .fold(outline("leaf", 1, Vec::new()), |child, i| {
                outline(&format!("n{i}"), 1, vec![child])
            });
        let toc = toc_from_outlines(&[chain], 3, 0);
        assert_eq!(
            tree_depth(&toc),
            MAX_OUTLINE_DEPTH + 1,
            "nothing past depth MAX renders, however deep the chain"
        );
    }

    #[test]
    fn dest_top_rejects_non_finite_and_missing_tops() {
        let kind = |top: f32| DestinationKind::XYZ {
            left: None,
            top: Some(top),
            zoom: None,
        };
        assert_eq!(dest_top(&kind(f32::NAN)), None);
        assert_eq!(dest_top(&kind(f32::INFINITY)), None);
        assert_eq!(dest_top(&kind(f32::NEG_INFINITY)), None);
        assert_eq!(dest_top(&kind(72.0)), Some(72.0));
        assert_eq!(dest_top(&DestinationKind::Fit), None);
        assert_eq!(
            dest_top(&DestinationKind::FitH { top: None }),
            None,
            "missing vertical component"
        );
    }

    #[test]
    fn preflight_rejects_directories() {
        let dir = std::env::temp_dir().join(format!("candi-mupdf-dir-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let result = preflight_path(dir.to_str().unwrap());
        assert!(matches!(result, Err(Error::NotFound(_))), "{result:?}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preflight_rejects_named_pipes_without_blocking() {
        let dir = std::env::temp_dir().join(format!("candi-mupdf-fifo-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let fifo = dir.join("pipe");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .expect("mkfifo exists")
                .success()
        );
        let started = Instant::now();
        let result = preflight_path(fifo.to_str().unwrap());
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "a FIFO must be rejected, not opened (which would block)"
        );
        match result {
            Err(Error::Other(msg)) if msg.contains("not a regular file") => {}
            other => panic!("expected a FIFO rejection, got {other:?}"),
        }
        fs::remove_dir_all(&dir).ok();
    }
}
