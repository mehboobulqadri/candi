// SPDX-License-Identifier: AGPL-3.0

use std::fs;
use std::path::{Path, PathBuf};

use candi_core::{
    Bookmark, Error, SessionLoad, SessionState, ZoomMode, load_session, save_session, sidecar_path,
};

fn temp_fixture_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "candi-session-test-{}-{}-{}",
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

fn pdf_path(dir: &Path) -> PathBuf {
    let path = dir.join("book.pdf");
    fs::write(&path, b"%PDF-1.4 dummy").unwrap();
    path
}

fn loaded(load: SessionLoad) -> SessionState {
    match load {
        SessionLoad::Loaded(session) => session,
        other => panic!("expected Loaded, got {other:?}"),
    }
}

fn corrupt_message(load: SessionLoad) -> String {
    match load {
        SessionLoad::Corrupt(message) => message,
        other => panic!("expected Corrupt, got {other:?}"),
    }
}

fn write_sidecar(pdf: &Path, contents: &str) {
    fs::write(sidecar_path(pdf), contents).unwrap();
}

#[test]
fn session_v2_roundtrip_all_fields() {
    let dir = temp_fixture_dir("v2-roundtrip");
    let pdf = pdf_path(&dir);
    let session = SessionState {
        page: 3,
        scroll_frac: 0.42,
        zoom: ZoomMode::FitWidth,
        theme: "Sepia".to_owned(),
        bookmarks: vec![Bookmark {
            page: 5,
            label: Some("intro".to_owned()),
            created_at: "2026-08-20T12:00:00Z".to_owned(),
        }],
    };

    save_session(&pdf, &session).unwrap();

    let sidecar = fs::read_to_string(sidecar_path(&pdf)).unwrap();
    assert!(sidecar.contains("schema_version = 2"), "sidecar: {sidecar}");
    assert!(loaded(load_session(&pdf).unwrap()) == session);
}

#[test]
fn zoom_percent_roundtrips() {
    let dir = temp_fixture_dir("zoom-percent");
    let pdf = pdf_path(&dir);
    let session = SessionState {
        zoom: ZoomMode::Percent(120),
        ..SessionState::new(10)
    };

    save_session(&pdf, &session).unwrap();
    assert_eq!(
        loaded(load_session(&pdf).unwrap()).zoom,
        ZoomMode::Percent(120)
    );
}

#[test]
fn label_less_bookmark_survives_roundtrip() {
    let dir = temp_fixture_dir("label-less");
    let pdf = pdf_path(&dir);
    let mut session = SessionState::new(10);
    session.bookmarks.push(Bookmark {
        page: 7,
        label: None,
        created_at: "2026-08-20T12:00:00Z".to_owned(),
    });

    save_session(&pdf, &session).unwrap();

    let restored = loaded(load_session(&pdf).unwrap());
    assert_eq!(restored.bookmarks.len(), 1);
    assert_eq!(restored.bookmarks[0].page, 7);
    assert_eq!(restored.bookmarks[0].label, None);
    assert_eq!(restored.bookmarks[0].created_at, "2026-08-20T12:00:00Z");
}

#[test]
fn v1_sidecar_migrates_with_defaults() {
    let dir = temp_fixture_dir("v1-migrate");
    let pdf = pdf_path(&dir);
    write_sidecar(
        &pdf,
        r#"schema_version = 1
[reading]
page = 7
scroll = 3
updated_at = "2026-08-20T12:00:00Z"
"#,
    );

    let session = loaded(load_session(&pdf).unwrap());
    assert_eq!(session.page, 7);
    assert_eq!(session.scroll_frac, 0.0);
    assert_eq!(session.zoom, ZoomMode::FitWidth);
    assert_eq!(session.theme, "Light");
    assert!(session.bookmarks.is_empty());
}

#[test]
fn garbage_returns_corrupt() {
    let dir = temp_fixture_dir("garbage");
    let pdf = pdf_path(&dir);
    write_sidecar(&pdf, "not valid {{{ toml");

    match load_session(&pdf).unwrap() {
        SessionLoad::Corrupt(message) => assert!(!message.is_empty()),
        other => panic!("expected Corrupt, got {other:?}"),
    }
}

#[test]
fn schema_version_three_is_unsupported() {
    let dir = temp_fixture_dir("schema-v3");
    let pdf = pdf_path(&dir);
    write_sidecar(
        &pdf,
        r#"schema_version = 3
updated_at = "2026-08-20T12:00:00Z"
[reading]
page = 0
scroll_frac = 0.0
zoom = "fit-width"
theme = "Light"
"#,
    );

    match load_session(&pdf) {
        Err(Error::UnsupportedSchema { found: 3 }) => {}
        other => panic!("expected UnsupportedSchema {{ found: 3 }}, got {other:?}"),
    }
}

#[test]
fn absurd_schema_version_reports_the_written_value() {
    let dir = temp_fixture_dir("schema-huge");
    let pdf = pdf_path(&dir);
    write_sidecar(&pdf, "schema_version = 4294967296\n");

    match load_session(&pdf) {
        Err(Error::UnsupportedSchema { found: 4294967296 }) => {}
        other => panic!("expected UnsupportedSchema {{ found: 4294967296 }}, got {other:?}"),
    }
}

#[test]
fn negative_schema_version_is_corrupt() {
    let dir = temp_fixture_dir("negative-version");
    let pdf = pdf_path(&dir);
    write_sidecar(&pdf, "schema_version = -1\n");

    assert_eq!(
        corrupt_message(load_session(&pdf).unwrap()),
        "negative schema_version"
    );
}

#[test]
fn missing_schema_version_is_corrupt() {
    let dir = temp_fixture_dir("missing-version");
    let pdf = pdf_path(&dir);
    write_sidecar(&pdf, "[reading]\npage = 0\n");

    assert_eq!(
        corrupt_message(load_session(&pdf).unwrap()),
        "missing schema_version"
    );
}

#[test]
fn missing_session_file_is_missing() {
    let dir = temp_fixture_dir("missing-file");
    let pdf = pdf_path(&dir);

    assert!(matches!(load_session(&pdf).unwrap(), SessionLoad::Missing));
}

#[test]
fn new_uses_document_defaults() {
    let session = SessionState::new(10);
    assert_eq!(session.page, 0);
    assert_eq!(session.scroll_frac, 0.0);
    assert_eq!(session.zoom, ZoomMode::FitWidth);
    assert_eq!(session.theme, "Light");
    assert!(session.bookmarks.is_empty());

    assert_eq!(SessionState::new(0), SessionState::new(1));
}

#[test]
fn clamp_to_bounds_page_and_fraction() {
    let beyond = SessionState {
        page: 9,
        ..SessionState::new(10)
    }
    .clamp_to(4);
    assert_eq!(beyond.page, 3);
    assert_eq!(SessionState::new(10).clamp_to(0).page, 0);

    let below = SessionState {
        scroll_frac: -0.5,
        ..SessionState::new(10)
    }
    .clamp_to(10);
    assert_eq!(below.scroll_frac, 0.0);

    let above = SessionState {
        scroll_frac: 1.5,
        ..SessionState::new(10)
    }
    .clamp_to(10);
    assert_eq!(above.scroll_frac, 1.0);

    let non_finite = SessionState {
        scroll_frac: f64::NAN,
        ..SessionState::new(10)
    }
    .clamp_to(10);
    assert_eq!(non_finite.scroll_frac, 0.0);
}

#[test]
fn clamp_to_drops_bookmarks_past_the_document() {
    let mut session = SessionState::new(10);
    session.add_bookmark(1);
    session.add_bookmark(9);

    let clamped = session.clamp_to(4);
    assert_eq!(
        clamped.bookmarks.iter().map(|b| b.page).collect::<Vec<_>>(),
        vec![1],
        "bookmarks past the last page are dropped"
    );
    assert!(SessionState::new(10).clamp_to(0).bookmarks.is_empty());
}

#[test]
fn absurd_zoom_percent_clamps_into_supported_range() {
    let dir = temp_fixture_dir("zoom-clamp");
    let pdf = pdf_path(&dir);

    for (stored, expected) in [
        ("5000", ZoomMode::Percent(candi_core::MAX_ZOOM_PERCENT)),
        ("1", ZoomMode::Percent(candi_core::MIN_ZOOM_PERCENT)),
        ("120", ZoomMode::Percent(120)),
    ] {
        write_sidecar(
            &pdf,
            &format!(
                r#"schema_version = 2
updated_at = "2026-08-20T12:00:00Z"
[reading]
page = 0
scroll_frac = 0.0
zoom = {stored}
theme = "Light"
"#
            ),
        );
        assert_eq!(
            loaded(load_session(&pdf).unwrap()).zoom,
            expected,
            "{stored}"
        );
    }
}

#[test]
fn bookmarks_dedup_by_page_and_toggle() {
    let mut session = SessionState::new(10);

    session.add_bookmark(5);
    session.add_bookmark(5);
    assert_eq!(session.bookmarks.len(), 1);

    session.toggle_bookmark(5);
    assert!(session.bookmarks.is_empty());

    session.toggle_bookmark(5);
    assert_eq!(session.bookmarks.len(), 1);
    assert_eq!(session.bookmarks[0].page, 5);
    assert!(session.bookmarks[0].created_at.ends_with('Z'));

    session.remove_bookmark(9);
    assert_eq!(session.bookmarks.len(), 1);

    session.remove_bookmark(5);
    assert!(session.bookmarks.is_empty());
}
