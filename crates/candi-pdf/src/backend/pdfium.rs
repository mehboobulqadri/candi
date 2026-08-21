// SPDX-License-Identifier: AGPL-3.0

//! PDFium engine behind the `pdfium-backend` feature.

use std::collections::HashSet;
use std::io;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex, OnceLock};

use pdfium_render::prelude::*;

use crate::{Backend, Block, Document, Error, Line, PageImage, PagePositions, TocItem, Word};

const ZERO_PAGE_MALFORMED: &str = "truncated or empty document";
const FPDF_ERR_FILE: u32 = 2;
const FPDF_ERR_FORMAT: u32 = 3;
const FPDF_ERR_PASSWORD: u32 = 4;
/// Render annotations; matches pdfium-render's default `PdfRenderConfig`.
const FPDF_ANNOT: std::ffi::c_int = 1;
const OPAQUE_WHITE: std::ffi::c_ulong = 0xFFFF_FFFF;

static ENGINE: OnceLock<Result<Arc<Pdfium>, Error>> = OnceLock::new();
static PDFIUM_OPS: Mutex<()> = Mutex::new(());

/// PDFium-backed document engine.
#[derive(Debug, Default)]
pub struct PdfiumBackend;

impl Backend for PdfiumBackend {
    fn name(&self) -> &'static str {
        "pdfium"
    }

    fn open(&self, path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error> {
        preflight_path(path)?;

        let (pdfium, handle, page_count) = {
            let _guard = pdfium_lock();
            let pdfium = shared_engine()?;
            let bindings = pdfium.bindings();

            let handle = bindings.FPDF_LoadDocument(path, password);
            if handle.is_null() {
                return Err(map_load_error(bindings, path, password));
            }

            let page_count = bindings.FPDF_GetPageCount(handle);
            if page_count < 0 {
                bindings.FPDF_CloseDocument(handle);
                return Err(Error::Other("page count unavailable".into()));
            }

            let page_count = usize::try_from(page_count)
                .map_err(|_| Error::Other("page count out of range".into()))?;

            if page_count == 0 {
                bindings.FPDF_CloseDocument(handle);
                return Err(Error::Malformed(ZERO_PAGE_MALFORMED.into()));
            }

            (pdfium, handle, page_count)
        };

        let document = Box::new(PdfiumPdfDocument {
            pdfium,
            handle,
            page_count,
        });
        crate::textlayer::reject_if_no_text_layer(document.as_ref())?;
        Ok(document)
    }
}

struct PdfiumPdfDocument {
    pdfium: Arc<Pdfium>,
    handle: FPDF_DOCUMENT,
    page_count: usize,
}

// Pdfium access is serialized by pdfium-render's `thread_safe` bindings; the raw
// document handle is only used through those bindings.
unsafe impl Send for PdfiumPdfDocument {}
unsafe impl Sync for PdfiumPdfDocument {}

impl Drop for PdfiumPdfDocument {
    fn drop(&mut self) {
        let _guard = pdfium_lock();
        if !self.handle.is_null() {
            self.pdfium.bindings().FPDF_CloseDocument(self.handle);
            self.handle = null_mut();
        }
    }
}

impl Document for PdfiumPdfDocument {
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_text(&self, page: usize) -> Result<String, Error> {
        let _guard = pdfium_lock();
        let page_index = page_index(page, self.page_count)?;
        with_page(self, page_index, |bindings, page_handle| {
            let text_page = load_text_page(bindings, page_handle)?;
            let result = extract_text_page_text(bindings, text_page);
            bindings.FPDFText_ClosePage(text_page);
            result
        })
    }

    fn page_positions(&self, page: usize) -> Result<Option<PagePositions>, Error> {
        let _guard = pdfium_lock();
        let page_index = page_index(page, self.page_count)?;
        with_page(self, page_index, |bindings, page_handle| {
            let text_page = load_text_page(bindings, page_handle)?;
            let positions = positions_from_page(bindings, page_handle, text_page);
            bindings.FPDFText_ClosePage(text_page);
            positions
        })
        .map(Some)
    }

    fn page_size(&self, page: usize) -> Result<(f32, f32), Error> {
        let _guard = pdfium_lock();
        let page_index = page_index(page, self.page_count)?;
        with_page(self, page_index, |bindings, page_handle| {
            Ok((
                bindings.FPDF_GetPageWidthF(page_handle),
                bindings.FPDF_GetPageHeightF(page_handle),
            ))
        })
    }

    // `FPDFBitmap_Create` always yields a 4-bytes-per-pixel BGRx/BGRA buffer
    // (see the pdfium-render bindings docs), so normalization to RGBA is a
    // channel swap plus forced opacity.
    fn render_page(&self, page: usize, scale: f32) -> Result<PageImage, Error> {
        let _guard = pdfium_lock();
        let page_index = page_index(page, self.page_count)?;
        with_page(self, page_index, |bindings, page_handle| {
            let width_pt = bindings.FPDF_GetPageWidthF(page_handle);
            let height_pt = bindings.FPDF_GetPageHeightF(page_handle);
            let width = (width_pt * scale).round() as i32;
            let height = (height_pt * scale).round() as i32;

            let bitmap = bindings.FPDFBitmap_Create(width, height, 0);
            if bitmap.is_null() {
                return Err(Error::Other(format!(
                    "could not allocate {width}x{height} render bitmap"
                )));
            }

            bindings.FPDFBitmap_FillRect(bitmap, 0, 0, width, height, OPAQUE_WHITE);
            bindings.FPDF_RenderPageBitmap(bitmap, page_handle, 0, 0, width, height, 0, FPDF_ANNOT);

            let mut buffer = bindings.FPDFBitmap_GetBuffer_as_vec(bitmap);
            bindings.FPDFBitmap_Destroy(bitmap);

            for pixel in buffer.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                pixel[3] = u8::MAX;
            }

            PageImage::from_rgba(width as u32, height as u32, buffer)
        })
    }

    fn outline(&self) -> Result<Vec<TocItem>, Error> {
        let _guard = pdfium_lock();
        let mut seen = HashSet::new();
        Ok(bookmark_children(
            self.handle,
            null_mut(),
            self.pdfium.bindings(),
            &mut seen,
        ))
    }
}

/// Converts the bookmark tree rooted at `parent` (`null` for the document
/// root). Pdfium's bookmark API signals failure only through null handles and
/// `-1` page indexes, so entries without a usable internal destination are
/// skipped; there is no error channel that could carry a whole-tree failure.
/// The visited set terminates cycles, which malformed documents can form
/// through repeated `/Next` or `/First` references.
fn bookmark_children(
    doc: FPDF_DOCUMENT,
    parent: FPDF_BOOKMARK,
    bindings: &dyn PdfiumLibraryBindings,
    seen: &mut HashSet<usize>,
) -> Vec<TocItem> {
    let mut items = Vec::new();
    let mut next = bindings.FPDFBookmark_GetFirstChild(doc, parent);
    while !next.is_null() && seen.insert(next as usize) {
        if let (Some(title), Some(page)) = (
            bookmark_title(bindings, next),
            bookmark_page(doc, next, bindings),
        ) {
            items.push(TocItem {
                title,
                page,
                children: bookmark_children(doc, next, bindings, seen),
            });
        }
        next = bindings.FPDFBookmark_GetNextSibling(doc, next);
    }
    items
}

fn bookmark_title(bindings: &dyn PdfiumLibraryBindings, bookmark: FPDF_BOOKMARK) -> Option<String> {
    let buffer_length = bindings.FPDFBookmark_GetTitle(bookmark, null_mut(), 0);
    if buffer_length == 0 {
        return None;
    }

    let mut buffer = vec![0u8; buffer_length as usize];
    bindings.FPDFBookmark_GetTitle(bookmark, buffer.as_mut_ptr().cast(), buffer_length);
    Some(decode_utf16_bytes(&buffer))
}

/// Resolves a bookmark's target as a 1-based page number. Bookmarks either
/// carry a direct `/Dest` or a `/A` action; hyperref-generated documents use
/// GoTo actions almost exclusively.
fn bookmark_page(
    doc: FPDF_DOCUMENT,
    bookmark: FPDF_BOOKMARK,
    bindings: &dyn PdfiumLibraryBindings,
) -> Option<usize> {
    let mut dest = bindings.FPDFBookmark_GetDest(doc, bookmark);
    if dest.is_null() {
        let action = bindings.FPDFBookmark_GetAction(bookmark);
        if !action.is_null()
            && bindings.FPDFAction_GetType(action)
                == PdfActionType::GoToDestinationInSameDocument as std::ffi::c_ulong
        {
            dest = bindings.FPDFAction_GetDest(doc, action);
        }
    }

    if dest.is_null() {
        return None;
    }
    usize::try_from(bindings.FPDFDest_GetDestPageIndex(doc, dest))
        .ok()
        .map(|index| index + 1)
}

fn pdfium_lock() -> std::sync::MutexGuard<'static, ()> {
    PDFIUM_OPS.lock().expect("pdfium operations mutex poisoned")
}

fn shared_engine() -> Result<Arc<Pdfium>, Error> {
    match ENGINE
        .get_or_init(|| bind_pdfium_library().map(|bindings| Arc::new(Pdfium::new(bindings))))
    {
        Ok(engine) => Ok(Arc::clone(engine)),
        Err(err) => Err(err.clone()),
    }
}

fn bind_pdfium_library() -> Result<Box<dyn PdfiumLibraryBindings>, Error> {
    if let Ok(dir) = std::env::var("PDFIUM_LIB") {
        let lib = Pdfium::pdfium_platform_library_name_at_path(&dir);
        if lib.exists() {
            return Pdfium::bind_to_library(&lib).map_err(map_bind_error);
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
        if lib.exists() {
            return Pdfium::bind_to_library(&lib).map_err(map_bind_error);
        }
    }

    Err(Error::Other(
        "libpdfium not found: set PDFIUM_LIB to the directory containing libpdfium.so, \
         or place the library next to the executable"
            .into(),
    ))
}

fn with_page<T>(
    doc: &PdfiumPdfDocument,
    page_index: u16,
    f: impl FnOnce(&dyn PdfiumLibraryBindings, FPDF_PAGE) -> Result<T, Error>,
) -> Result<T, Error> {
    let bindings = doc.pdfium.bindings();
    let page_handle = bindings.FPDF_LoadPage(doc.handle, page_index.into());
    if page_handle.is_null() {
        return Err(Error::Other(format!(
            "page {page_index} could not be loaded"
        )));
    }

    let result = f(bindings, page_handle);
    bindings.FPDF_ClosePage(page_handle);
    result
}

fn load_text_page(
    bindings: &dyn PdfiumLibraryBindings,
    page_handle: FPDF_PAGE,
) -> Result<FPDF_TEXTPAGE, Error> {
    let text_page = bindings.FPDFText_LoadPage(page_handle);
    if text_page.is_null() {
        return Err(Error::Other("text page could not be loaded".into()));
    }
    Ok(text_page)
}

fn extract_text_page_text(
    bindings: &dyn PdfiumLibraryBindings,
    text_page: FPDF_TEXTPAGE,
) -> Result<String, Error> {
    let char_count = bindings.FPDFText_CountChars(text_page);
    if char_count <= 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; char_count as usize + 1];
    let written = bindings.FPDFText_GetText(text_page, 0, char_count, buffer.as_mut_ptr());
    if written <= 0 {
        return Ok(String::new());
    }

    Ok(decode_utf16(&buffer[..written as usize]))
}

fn positions_from_page(
    bindings: &dyn PdfiumLibraryBindings,
    page_handle: FPDF_PAGE,
    text_page: FPDF_TEXTPAGE,
) -> Result<PagePositions, Error> {
    let object_count = bindings.FPDFPage_CountObjects(page_handle);
    if object_count <= 0 {
        return Ok(PagePositions { blocks: Vec::new() });
    }

    let mut blocks = Vec::new();

    for object_index in 0..object_count {
        let object = bindings.FPDFPage_GetObject(page_handle, object_index);
        if object.is_null() {
            continue;
        }

        if bindings.FPDFPageObj_GetType(object) != FPDF_PAGEOBJ_TEXT as i32 {
            continue;
        }

        let Some((left, bottom, _right, top)) = object_bounds(bindings, object) else {
            continue;
        };

        let text = text_object_text(bindings, object, text_page);
        if text.is_empty() {
            continue;
        }

        let font_size = (top - bottom).max(0.0);
        let mut lines = segment_lines_for_object(bindings, text_page, object, font_size);
        if lines.is_empty() {
            let words = whitespace_words(&text, left, bottom, font_size);
            if !words.is_empty() {
                lines.push(Line { words });
            }
        }

        if !lines.is_empty() {
            blocks.push(Block { lines });
        } else if !text.trim().is_empty() {
            blocks.push(Block {
                lines: vec![Line {
                    words: vec![Word {
                        text: text.trim().to_string(),
                        x: left,
                        y: bottom,
                        font_size,
                    }],
                }],
            });
        }
    }

    Ok(PagePositions { blocks })
}

fn segment_lines_for_object(
    bindings: &dyn PdfiumLibraryBindings,
    text_page: FPDF_TEXTPAGE,
    object: FPDF_PAGEOBJECT,
    font_size: f32,
) -> Vec<Line> {
    let char_count = bindings.FPDFText_CountChars(text_page);
    if char_count <= 0 {
        return Vec::new();
    }

    let rect_count = bindings.FPDFText_CountRects(text_page, 0, char_count) as usize;
    let mut lines = Vec::new();

    for rect_index in 0..rect_count {
        let mut left = 0.0f64;
        let mut top = 0.0f64;
        let mut right = 0.0f64;
        let mut bottom = 0.0f64;
        if !bindings.is_true(bindings.FPDFText_GetRect(
            text_page,
            rect_index as i32,
            &mut left,
            &mut top,
            &mut right,
            &mut bottom,
        )) {
            continue;
        }

        if !segment_belongs_to_object(bindings, text_page, object, rect_index as i32) {
            continue;
        }

        let segment_text = bounded_text(bindings, text_page, left, top, right, bottom);
        if segment_text.trim().is_empty() {
            continue;
        }

        let words = whitespace_words(&segment_text, left as f32, bottom as f32, font_size);
        if !words.is_empty() {
            lines.push(Line { words });
        }
    }

    lines
}

fn segment_belongs_to_object(
    bindings: &dyn PdfiumLibraryBindings,
    text_page: FPDF_TEXTPAGE,
    object: FPDF_PAGEOBJECT,
    rect_index: i32,
) -> bool {
    let mut left = 0.0f64;
    let mut top = 0.0f64;
    let mut right = 0.0f64;
    let mut bottom = 0.0f64;
    if !bindings.is_true(bindings.FPDFText_GetRect(
        text_page,
        rect_index,
        &mut left,
        &mut top,
        &mut right,
        &mut bottom,
    )) {
        return false;
    }

    let char_count = bindings.FPDFText_CountChars(text_page);
    for char_index in 0..char_count {
        let mut char_left = 0.0f64;
        let mut char_right = 0.0f64;
        let mut char_bottom = 0.0f64;
        let mut char_top = 0.0f64;
        if !bindings.is_true(bindings.FPDFText_GetCharBox(
            text_page,
            char_index,
            &mut char_left,
            &mut char_right,
            &mut char_bottom,
            &mut char_top,
        )) {
            continue;
        }

        let in_rect =
            char_left >= left && char_right <= right && char_bottom >= bottom && char_top <= top;
        if !in_rect {
            continue;
        }

        let char_object = bindings.FPDFText_GetTextObject(text_page, char_index);
        if char_object == object {
            return true;
        }
    }

    false
}

fn bounded_text(
    bindings: &dyn PdfiumLibraryBindings,
    text_page: FPDF_TEXTPAGE,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
) -> String {
    let chars_count =
        bindings.FPDFText_GetBoundedText(text_page, left, top, right, bottom, null_mut(), 0);
    if chars_count <= 0 {
        return String::new();
    }

    let mut buffer = vec![0u16; chars_count as usize];
    let written = bindings.FPDFText_GetBoundedText(
        text_page,
        left,
        top,
        right,
        bottom,
        buffer.as_mut_ptr(),
        chars_count,
    );
    if written <= 0 {
        return String::new();
    }

    decode_utf16(&buffer[..written as usize])
}

fn decode_utf16(buffer: &[u16]) -> String {
    String::from_utf16_lossy(buffer)
        .trim_end_matches('\0')
        .to_string()
}

fn decode_utf16_bytes(buffer: &[u8]) -> String {
    let mut units = Vec::with_capacity(buffer.len() / 2);
    for chunk in buffer.chunks_exact(2) {
        units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    decode_utf16(&units)
}

fn whitespace_words(text: &str, x: f32, y: f32, font_size: f32) -> Vec<Word> {
    text.split_whitespace()
        .map(|word| Word {
            text: word.to_string(),
            x,
            y,
            font_size,
        })
        .collect()
}

fn object_bounds(
    bindings: &dyn PdfiumLibraryBindings,
    object: FPDF_PAGEOBJECT,
) -> Option<(f32, f32, f32, f32)> {
    let mut left = 0.0f32;
    let mut bottom = 0.0f32;
    let mut right = 0.0f32;
    let mut top = 0.0f32;
    if bindings.is_true(bindings.FPDFPageObj_GetBounds(
        object,
        &mut left,
        &mut bottom,
        &mut right,
        &mut top,
    )) {
        Some((left, bottom, right, top))
    } else {
        None
    }
}

fn text_object_text(
    bindings: &dyn PdfiumLibraryBindings,
    object: FPDF_PAGEOBJECT,
    text_page: FPDF_TEXTPAGE,
) -> String {
    let buffer_length = bindings.FPDFTextObj_GetText(object, text_page, null_mut(), 0);
    if buffer_length == 0 {
        return String::new();
    }

    let mut buffer = vec![0u8; buffer_length as usize];
    let written = bindings.FPDFTextObj_GetText(
        object,
        text_page,
        buffer.as_mut_ptr() as *mut FPDF_WCHAR,
        buffer_length,
    );
    if written == 0 {
        return String::new();
    }

    decode_utf16_bytes(&buffer)
}

fn page_index(page: usize, page_count: usize) -> Result<u16, Error> {
    if page >= page_count {
        return Err(Error::Other(format!(
            "page {page} out of range ({page_count} pages)"
        )));
    }
    u16::try_from(page).map_err(|_| Error::Other(format!("page index {page} out of range")))
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

fn map_bind_error(err: PdfiumError) -> Error {
    match err {
        PdfiumError::LoadLibraryError(load_err) => {
            Error::Other(format!("failed to load libpdfium: {load_err}"))
        }
        other => Error::Other(other.to_string()),
    }
}

fn fpdf_last_error(bindings: &dyn PdfiumLibraryBindings) -> u32 {
    #[cfg(target_os = "windows")]
    {
        bindings.FPDF_GetLastError()
    }
    #[cfg(not(target_os = "windows"))]
    {
        bindings.FPDF_GetLastError() as u32
    }
}

fn map_load_error(
    bindings: &dyn PdfiumLibraryBindings,
    path: &str,
    password: Option<&str>,
) -> Error {
    let code = fpdf_last_error(bindings);
    match code {
        FPDF_ERR_FORMAT if is_zero_page_catalog(path) => {
            Error::Malformed(ZERO_PAGE_MALFORMED.into())
        }
        FPDF_ERR_FORMAT => Error::Malformed("invalid PDF document".into()),
        FPDF_ERR_PASSWORD => {
            if password.is_some() {
                Error::WrongPassword("password rejected".into())
            } else {
                Error::Encrypted("document requires a password".into())
            }
        }
        FPDF_ERR_FILE => map_file_error(path),
        _ => Error::Other(format!("FPDF_LoadDocument failed (FPDF_ERR code {code})")),
    }
}

fn is_zero_page_catalog(path: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let text = String::from_utf8_lossy(&bytes);
    text.contains("/Kids [] /Count 0") || text.contains("/Kids[] /Count 0")
}

fn map_file_error(path: &str) -> Error {
    match std::fs::metadata(path) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Error::NotFound(err.to_string()),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            Error::PermissionDenied(err.to_string())
        }
        Err(err) => Error::Other(err.to_string()),
        Ok(_) => Error::Other(format!("could not read PDF file at {path}")),
    }
}

fn map_io_error(err: io::Error) -> Error {
    match err.kind() {
        io::ErrorKind::NotFound => Error::NotFound(err.to_string()),
        io::ErrorKind::PermissionDenied => Error::PermissionDenied(err.to_string()),
        _ => Error::Other(err.to_string()),
    }
}
