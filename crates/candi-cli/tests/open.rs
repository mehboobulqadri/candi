// SPDX-License-Identifier: AGPL-3.0

use std::path::PathBuf;

use candi_cli::{open_document, parse_backend, view_from_load};
use candi_core::{Load, ViewState, normalize_reader_text};
use candi_pdf::{Backend, BackendKind};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../candi-pdf/tests/fixtures/{name}"))
}

#[test]
fn open_tiny_pdf_loads_first_page() {
    let path = fixture("tiny.pdf");
    let opened = open_document(&path, BackendKind::Mupdf).unwrap();
    assert_eq!(opened.view.page(), 0);
    assert!(opened.document.page_count() > 0);
}

#[test]
fn parse_backend_rejects_unknown() {
    assert!(parse_backend("nope").is_err());
}

#[test]
fn normalize_ligatures_in_extracted_text() {
    let path = fixture("tiny.pdf");
    let opened = open_document(&path, BackendKind::Mupdf).unwrap();
    let raw = opened.document.page_text(0).unwrap();
    let normalized = normalize_reader_text(&raw);
    assert!(!normalized.is_empty());
}

#[test]
fn corrupt_sidecar_starts_fresh() {
    let backend = candi_pdf::stub::StubBackend::new(3);
    let doc = backend.open("x.pdf", None).unwrap();
    let view = view_from_load(doc.as_ref(), Load::Corrupt("bad".into()));
    assert_eq!(view, ViewState::new());
}
