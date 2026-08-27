// SPDX-License-Identifier: AGPL-3.0

use std::fs;
use std::path::PathBuf;

use candi_cli::{
    OpenSession, open_document, open_session, parse_backend, save_session, view_from_load,
};
use candi_core::{Load, SessionState, ViewState, ZoomMode, normalize_reader_text, sidecar_path};
use candi_pdf::{Backend, BackendKind};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../candi-pdf/tests/fixtures/{name}"))
}

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "candi-cli-test-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn copied_fixture(name: &str, label: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(label);
    let pdf = dir.join(name);
    fs::copy(fixture(name), &pdf).unwrap();
    (dir, pdf)
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

#[test]
fn open_session_migrates_preseeded_v1_sidecar() {
    let (_dir, pdf) = copied_fixture("blank-first-page.pdf", "session-v1");
    fs::write(
        sidecar_path(&pdf),
        r#"schema_version = 1
[reading]
page = 1
scroll = 5
updated_at = "2026-08-20T12:00:00Z"
"#,
    )
    .unwrap();

    let OpenSession {
        document, session, ..
    } = open_session(&pdf, BackendKind::Mupdf).unwrap();
    assert_eq!(session.page, 1);
    assert_eq!(session.scroll_frac, 0.0);
    assert_eq!(session.zoom, ZoomMode::FitWidth);
    assert_eq!(session.theme, "Dark");
    assert!(session.bookmarks.is_empty());
    assert!(document.page_count() > 1);
}

#[test]
fn save_session_writes_schema_version_three() {
    let (_dir, pdf) = copied_fixture("tiny.pdf", "session-save");

    save_session(&pdf, &SessionState::new(2)).unwrap();

    let sidecar = fs::read_to_string(sidecar_path(&pdf)).unwrap();
    assert!(sidecar.contains("schema_version = 3"), "sidecar: {sidecar}");
}

#[test]
fn saved_session_round_trips_through_open_session() {
    let (_dir, pdf) = copied_fixture("blank-first-page.pdf", "session-roundtrip");
    let session = SessionState {
        page: 1,
        scroll_frac: 0.25,
        zoom: ZoomMode::Percent(90),
        theme: "Sepia".to_owned(),
        bookmarks: Vec::new(),
    };

    save_session(&pdf, &session).unwrap();

    let opened = open_session(&pdf, BackendKind::Mupdf).unwrap();
    assert_eq!(opened.session, session);
}

#[test]
fn corrupt_sidecar_reports_a_warning_and_a_fresh_session() {
    let (_dir, pdf) = copied_fixture("tiny.pdf", "session-corrupt");
    fs::write(sidecar_path(&pdf), "not valid {{{ toml").unwrap();

    let opened = open_session(&pdf, BackendKind::Mupdf).unwrap();

    let warning = opened.warning.expect("corruption must be reported");
    assert!(!warning.is_empty());
    assert_eq!(
        opened.session,
        SessionState::new(opened.document.page_count()),
        "the fresh session must not carry stale data"
    );
}

#[test]
fn missing_sidecar_carries_no_warning() {
    let (_dir, pdf) = copied_fixture("tiny.pdf", "session-missing");

    let opened = open_session(&pdf, BackendKind::Mupdf).unwrap();

    assert_eq!(opened.warning, None);
}
