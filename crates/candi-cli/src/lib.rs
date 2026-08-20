// SPDX-License-Identifier: AGPL-3.0

//! Shared open/resume/save helpers for Candi frontends.

use std::path::Path;

use candi_core::{Load, Position, ViewState, load, save};
use candi_pdf::{BackendKind, Document, Error as PdfError, open};

/// Scroll cap before the UI knows wrap height; page max scroll is unknown until then.
pub const UNBOUND_SCROLL: usize = usize::MAX;

pub struct OpenDocument {
    pub document: Box<dyn Document>,
    pub view: ViewState,
}

#[derive(Debug)]
pub enum OpenError {
    Pdf(PdfError),
    Core(candi_core::Error),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdf(err) => write!(f, "{err}"),
            Self::Core(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for OpenError {}

pub fn parse_backend(name: &str) -> Result<BackendKind, PdfError> {
    match name {
        "mupdf" => Ok(BackendKind::Mupdf),
        "pdfium" => Ok(BackendKind::Pdfium),
        other => Err(PdfError::Unsupported(format!("backend '{other}'"))),
    }
}

pub fn open_document(path: &Path, kind: BackendKind) -> Result<OpenDocument, OpenError> {
    let pdf = open(kind, path.to_string_lossy().as_ref(), None).map_err(OpenError::Pdf)?;
    let view = match load(path) {
        Ok(load) => view_from_load(pdf.as_ref(), load),
        Err(err) => return Err(OpenError::Core(err)),
    };
    Ok(OpenDocument {
        document: pdf,
        view,
    })
}

pub fn view_from_load(pdf: &dyn Document, load: Load) -> ViewState {
    match load {
        Load::Missing => ViewState::new(),
        Load::Loaded(position) => ViewState::new()
            .goto_page(position.page(), pdf.page_count())
            .scroll_down(position.scroll(), UNBOUND_SCROLL),
        Load::Corrupt(message) => {
            eprintln!("warning: corrupt sidecar: {message}");
            ViewState::new()
        }
    }
}

pub fn save_reading_position(path: &Path, view: ViewState) -> Result<(), candi_core::Error> {
    save(path, &Position::new(view.page(), view.scroll_offset(), ""))
}
