// SPDX-License-Identifier: AGPL-3.0

//! MuPDF backend fixture tests.

#![cfg(feature = "mupdf-backend")]

#[path = "common/outline.rs"]
mod outline;

use std::env;
use std::path::{Path, PathBuf};

use candi_pdf::{BackendKind, Error, open};

fn tiny_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.pdf")
}

fn zero_pages_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zero-pages.pdf")
}

fn blank_first_page_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/blank-first-page.pdf")
}

const ZERO_PAGE_GUARD_MESSAGE: &str = "truncated or empty document";

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
fn zero_page_count_after_open_hits_malformed_guard() {
    let fixture = zero_pages_fixture();
    let path = fixture.to_str().unwrap();
    assert!(matches!(
        open(BackendKind::Mupdf, path, None),
        Err(Error::Malformed(msg)) if msg == ZERO_PAGE_GUARD_MESSAGE
    ));
}

#[test]
fn truncated_pdf_may_fail_before_zero_page_guard() {
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
fn attention_paper_outline_when_present() {
    let Some(path) = attention_fixture() else {
        eprintln!(
            "SKIP: attention paper fixture absent (set CANDI_ATTENTION_PDF or add spikes/corpus/1706.03762-attention-is-all-you-need.pdf)"
        );
        return;
    };
    let path_str = path.to_str().unwrap();
    let doc = open(BackendKind::Mupdf, path_str, None).unwrap();
    assert_eq!(doc.outline().unwrap(), outline::attention_outline());
}

#[test]
fn empty_page_returns_empty_string() {
    let fixture = blank_first_page_fixture();
    let path = fixture.to_str().unwrap();
    let doc = open(BackendKind::Mupdf, path, None).unwrap();
    assert_eq!(doc.page_count(), 2);
    assert_eq!(doc.page_text(0).unwrap(), "");
    assert!(doc.page_text(1).unwrap().contains("Page two"));
}

fn assert_fixture_exists(path: &Path) {
    assert!(path.exists(), "expected fixture at {}", path.display());
}

fn silberschatz_fixture() -> Option<PathBuf> {
    if let Ok(path) = env::var("CANDI_SILBERSCHATZ_PDF") {
        let path = PathBuf::from(path);
        return path.exists().then_some(path);
    }
    None
}

fn rss_mb(field: &str) -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("status");
    status
        .lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
        / 1024
}

/// Optional manual probe for MuPDF silberschatz RSS during a full page sweep.
///
/// Findings (2026-08-20): without store intervention, VmRSS/VmHWM plateau near 295 MB
/// on 889 pages; `pdf_empty_store` + `fz_empty_store` after each page *increases*
/// peak (~940 MB) by forcing re-decode while MuPDF retains other per-page state.
/// Gentle `fz_shrink_store` alone does not cap full-pass peak; v0.1 budget gates
/// `reader_peak` (page window), not this full sweep.
///
/// Run: `CANDI_SILBERSCHATZ_PDF=/path/to/silberschatz.pdf cargo test -p candi-pdf --release --features mupdf-backend silberschatz_memory_plateau_probe -- --nocapture`
#[test]
fn silberschatz_memory_plateau_probe() {
    let Some(path) = silberschatz_fixture() else {
        eprintln!("SKIP: set CANDI_SILBERSCHATZ_PDF to run silberschatz memory probe");
        return;
    };
    let path = path.to_str().unwrap();
    let doc = open(BackendKind::Mupdf, path, None).expect("open");
    let pages = doc.page_count();
    eprintln!(
        "pages={pages} baseline rss={} hwm={}",
        rss_mb("VmRSS:"),
        rss_mb("VmHWM:")
    );
    for page in 0..pages {
        doc.page_text(page).expect("page_text");
        if page == 0 || page == pages / 2 || page + 1 == pages {
            eprintln!(
                "after page {page}: rss={} hwm={}",
                rss_mb("VmRSS:"),
                rss_mb("VmHWM:")
            );
        }
    }
    eprintln!("final rss={} hwm={}", rss_mb("VmRSS:"), rss_mb("VmHWM:"));
}

#[test]
fn committed_fixtures_exist() {
    assert_fixture_exists(&tiny_fixture());
    assert_fixture_exists(&zero_pages_fixture());
    assert_fixture_exists(&blank_first_page_fixture());
    assert_fixture_exists(&encrypted_fixture());
}
