// SPDX-License-Identifier: AGPL-3.0

//! Shared open/resume/save helpers for Candi frontends.

use std::path::Path;

use candi_core::{Load, Position, SessionLoad, SessionState, ViewState, load, load_session, save};
use candi_pdf::{BackendKind, Document, Error as PdfError, open};

pub use candi_core::save_session;

/// Scroll cap before the UI knows wrap height; page max scroll is unknown until then.
pub const UNBOUND_SCROLL: usize = usize::MAX;

pub struct OpenDocument {
    pub document: Box<dyn Document>,
    pub view: ViewState,
}

/// An open document paired with its full reading session (schema v2 sidecar).
/// A corrupt sidecar starts a fresh session and carries its message as
/// `warning` so a GUI can keep the unreadable file safe on disk.
pub struct OpenSession {
    pub document: Box<dyn Document>,
    pub session: SessionState,
    pub warning: Option<String>,
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

/// Open a document and its reading session, migrating v1 sidecars on the way.
///
/// Missing or corrupt sidecars start a fresh default session (corruption is
/// reported as a warning on stderr and in [`OpenSession::warning`]); pages
/// and scroll fraction are clamped to the document. Unsupported schema
/// versions are a hard error.
pub fn open_session(path: &Path, kind: BackendKind) -> Result<OpenSession, OpenError> {
    let pdf = open(kind, path.to_string_lossy().as_ref(), None).map_err(OpenError::Pdf)?;
    let (session, warning) = match load_session(path) {
        Ok(SessionLoad::Loaded(session)) => (session.clamp_to(pdf.page_count()), None),
        Ok(SessionLoad::Corrupt(message)) => {
            eprintln!("warning: corrupt sidecar: {message}");
            (SessionState::new(pdf.page_count()), Some(message))
        }
        Ok(SessionLoad::Missing) => (SessionState::new(pdf.page_count()), None),
        Err(err) => return Err(OpenError::Core(err)),
    };
    Ok(OpenSession {
        document: pdf,
        session,
        warning,
    })
}
