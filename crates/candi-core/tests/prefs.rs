// SPDX-License-Identifier: AGPL-3.0

//! Tolerant loading, atomic storing, and recents bookkeeping for the app
//! config file.

use std::fs;
use std::path::{Path, PathBuf};

use candi_core::{Prefs, Recent, load_prefs, store_prefs};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("candi-prefs-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn missing_file_yields_dark_defaults() {
    let dir = temp_dir("missing");
    let prefs = load_prefs(&dir.join("config.toml"));
    assert_eq!(prefs.theme, "Dark");
    assert!(prefs.recents.is_empty());
}

#[test]
fn corrupt_file_yields_defaults() {
    let dir = temp_dir("corrupt");
    let path = dir.join("config.toml");
    fs::write(&path, "not [valid toml").unwrap();
    let prefs = load_prefs(&path);
    assert_eq!(prefs, Prefs::default());
}

#[test]
fn newer_schema_yields_defaults() {
    let dir = temp_dir("newer");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "schema_version = 99\n\n[appearance]\ntheme = \"Sepia\"\n",
    )
    .unwrap();
    assert_eq!(load_prefs(&path), Prefs::default());
}

#[test]
fn partial_file_keeps_the_good_fields() {
    let dir = temp_dir("partial");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "schema_version = 1\n\n[[recents]]\npath = \"/tmp/a.pdf\"\nlast_opened = \"2026-08-27T00:00:00Z\"\n",
    )
    .unwrap();
    let prefs = load_prefs(&path);
    assert_eq!(prefs.theme, "Dark", "missing appearance falls back");
    assert_eq!(
        prefs.recents,
        vec![Recent {
            path: PathBuf::from("/tmp/a.pdf"),
            last_opened: "2026-08-27T00:00:00Z".into(),
        }]
    );
}

#[test]
fn malformed_recents_entries_are_skipped() {
    let dir = temp_dir("bad-recents");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "schema_version = 1\n\n[appearance]\ntheme = \"Sepia\"\n\n[[recents]]\npath = \"\"\nlast_opened = \"\"\n\n[[recents]]\npath = \"/tmp/b.pdf\"\nlast_opened = \"2026-08-27T01:00:00Z\"\n",
    )
    .unwrap();
    let prefs = load_prefs(&path);
    assert_eq!(prefs.theme, "Sepia");
    assert_eq!(prefs.recents.len(), 1);
    assert_eq!(prefs.recents[0].path, PathBuf::from("/tmp/b.pdf"));
}

#[test]
fn store_then_load_roundtrips() {
    let dir = temp_dir("roundtrip");
    let path = dir.join("nested").join("config.toml");
    let mut prefs = Prefs {
        theme: "Sepia".into(),
        ..Prefs::default()
    };
    prefs.record_open(PathBuf::from("/tmp/a.pdf").as_path());
    store_prefs(&path, &prefs).expect("store creates parents");
    assert_eq!(load_prefs(&path), prefs);
    assert!(
        !dir.join("nested").join("config.toml.tmp").exists(),
        "atomic write leaves no temp file"
    );
}

#[test]
fn record_open_dedupes_moves_to_front_and_caps() {
    let mut prefs = Prefs::default();
    for i in 0..12 {
        prefs.record_open(Path::new(&format!("/tmp/book-{i}.pdf")));
    }
    assert_eq!(prefs.recents.len(), 10);
    assert_eq!(
        prefs.recents[0].path,
        PathBuf::from("/tmp/book-11.pdf"),
        "most recent first"
    );
    assert_eq!(prefs.recents[9].path, PathBuf::from("/tmp/book-2.pdf"));

    prefs.record_open(Path::new("/tmp/book-5.pdf"));
    assert_eq!(prefs.recents.len(), 10);
    assert_eq!(prefs.recents[0].path, PathBuf::from("/tmp/book-5.pdf"));
    assert_eq!(prefs.recents[1].path, PathBuf::from("/tmp/book-11.pdf"));
    assert!(
        prefs.recents[1..]
            .iter()
            .all(|r| r.path.as_path() != Path::new("/tmp/book-5.pdf")),
        "deduped"
    );
    assert!(prefs.recents[0].last_opened.ends_with('Z'));
}

#[test]
fn store_failure_surfaces_as_err_not_panic() {
    let dir = temp_dir("fail");
    fs::write(dir.join("blocker"), "x").unwrap();
    let path = dir.join("blocker").join("config.toml");
    let err = store_prefs(&path, &Prefs::default());
    assert!(err.is_err(), "parent is a regular file");
}
