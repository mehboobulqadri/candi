// SPDX-License-Identifier: AGPL-3.0

//! MuPDF backend fixture tests.

#![cfg(feature = "mupdf-backend")]

use std::env;
use std::path::{Path, PathBuf};

use candi_pdf::{BackendKind, Error, open};

fn tiny_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.pdf")
}

fn encrypted_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bench/fixtures/dummy-encrypted.pdf")
}

fn attention_fixture() -> Option<PathBuf> {
    if let Ok(path) = env::var("CANDI_ATTENTION_PDF") {
        let path = PathBuf::from(path);
        return path.exists().then_some(path);
    }

    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spikes/corpus/1706.03762-attention-is-all-you-need.pdf");
    default.exists().then_some(default)
}

#[test]
fn tiny_pdf_opens_with_cached_page_count() {
    let path = tiny_fixture();
    let path = path.to_str().unwrap();
    let doc = open(BackendKind::Mupdf, path, None).unwrap();
    assert_eq!(doc.page_count(), 1);
    assert!(doc.page_text(0).unwrap().contains("Hello Candi"));
}

#[test]
fn tiny_pdf_positions_have_words() {
    let path = tiny_fixture().to_str().unwrap().to_string();
    let doc = open(BackendKind::Mupdf, &path, None).unwrap();
    let positions = doc.page_positions(0).unwrap().expect("mupdf has positions");
    assert!(!positions.blocks.is_empty());
    assert!(!positions.blocks[0].lines.is_empty());
    assert!(!positions.blocks[0].lines[0].words.is_empty());
}

#[test]
fn missing_file_is_not_found() {
    assert!(matches!(
        open(BackendKind::Mupdf, "/no/such/candi-fixture.pdf", None),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn encrypted_without_password_is_encrypted() {
    let path = encrypted_fixture().to_str().unwrap().to_string();
    assert!(matches!(
        open(BackendKind::Mupdf, &path, None),
        Err(Error::Encrypted(_))
    ));
}

#[test]
fn encrypted_with_wrong_password_is_wrong_password() {
    let path = encrypted_fixture().to_str().unwrap().to_string();
    assert!(matches!(
        open(BackendKind::Mupdf, &path, Some("bad")),
        Err(Error::WrongPassword(_))
    ));
}

#[test]
fn encrypted_with_correct_password_opens() {
    let path = encrypted_fixture().to_str().unwrap().to_string();
    let doc = open(BackendKind::Mupdf, &path, Some("123456")).unwrap();
    assert_eq!(doc.page_count(), 1);
}

#[test]
fn truncated_pdf_is_malformed() {
    let tiny = std::fs::read(tiny_fixture()).unwrap();
    let truncated = &tiny[..100.min(tiny.len())];
    let dir = env::temp_dir();
    let path = dir.join("candi-truncated.pdf");
    std::fs::write(&path, truncated).unwrap();
    let path_str = path.to_str().unwrap();
    assert!(matches!(
        open(BackendKind::Mupdf, path_str, None),
        Err(Error::Malformed(_))
    ));
    let _ = std::fs::remove_file(path);
}

#[test]
fn attention_paper_when_present() {
    let Some(path) = attention_fixture() else {
        eprintln!(
            "SKIP: attention paper fixture absent (set CANDI_ATTENTION_PDF or add spikes/corpus/1706.03762-attention-is-all-you-need.pdf)"
        );
        return;
    };
    let path_str = path.to_str().unwrap();
    let doc = open(BackendKind::Mupdf, path_str, None).unwrap();
    assert_eq!(doc.page_count(), 15);

    let first_text_page = (0..doc.page_count())
        .find(|&p| !doc.page_text(p).unwrap().trim().is_empty())
        .expect("expected at least one non-empty text page");

    let text = doc.page_text(first_text_page).unwrap();
    assert!(
        text.contains("Attention Is All You Need") || text.contains("Introduction"),
        "unexpected first text page snippet"
    );
}

#[test]
fn empty_page_returns_empty_string() {
    let Some(path) = attention_fixture() else {
        eprintln!("SKIP: attention paper fixture absent");
        return;
    };
    let doc = open(BackendKind::Mupdf, path.to_str().unwrap(), None).unwrap();
    if doc.page_text(0).unwrap().trim().is_empty() {
        assert_eq!(doc.page_text(0).unwrap(), "");
    }
}

fn assert_fixture_exists(path: &Path) {
    assert!(path.exists(), "expected fixture at {}", path.display());
}

#[test]
fn committed_fixtures_exist() {
    assert_fixture_exists(&tiny_fixture());
    assert_fixture_exists(&encrypted_fixture());
}
