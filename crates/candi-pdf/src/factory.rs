// SPDX-License-Identifier: AGPL-3.0

//! Backend registry and runtime selection.
//!
//! Features gate which backends are compiled in; this slice has no real
//! engines yet, so a compiled-in kind currently opens a [`StubBackend`]. The
//! stub wiring is replaced by the real engine in slices 01/02/01/03.

use crate::stub::StubBackend;
use crate::{Backend, Document, Error};

/// The engines candi knows how to talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Mupdf,
    Pdfium,
}

/// Names of the backends compiled into this build, in a stable order.
///
/// The CLI accepts these as `--backend <name>`; unknown names are rejected by
/// the CLI, not here.
// The pushes are cfg-gated, so a single vec![] literal cannot express this.
#[allow(clippy::vec_init_then_push)]
pub fn available() -> Vec<&'static str> {
    let mut names = Vec::new();
    #[cfg(feature = "mupdf-backend")]
    names.push("mupdf");
    #[cfg(feature = "pdfium-backend")]
    names.push("pdfium");
    names
}

/// Open a document with a specific backend.
///
/// A kind whose feature is not compiled in returns [`Error::Unsupported`];
/// this is the only path where the stub cannot stand in for the engine.
pub fn open(
    kind: BackendKind,
    path: &str,
    password: Option<&str>,
) -> Result<Box<dyn Document>, Error> {
    match kind {
        #[cfg(feature = "mupdf-backend")]
        BackendKind::Mupdf => StubBackend::new(1).open(path, password),
        #[cfg(feature = "pdfium-backend")]
        BackendKind::Pdfium => StubBackend::new(1).open(path, password),
        #[cfg(not(feature = "mupdf-backend"))]
        BackendKind::Mupdf => Err(Error::Unsupported(
            "mupdf-backend is not compiled in".into(),
        )),
        #[cfg(not(feature = "pdfium-backend"))]
        BackendKind::Pdfium => Err(Error::Unsupported(
            "pdfium-backend is not compiled in".into(),
        )),
    }
}

/// Open a document with the default backend ([`BackendKind::Mupdf`]).
///
/// The CLI's `--backend` default is mupdf; a future config key routes here
/// too. In a pdfium-only build this returns [`Error::Unsupported`].
pub fn open_default(path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error> {
    open(BackendKind::Mupdf, path, password)
}
