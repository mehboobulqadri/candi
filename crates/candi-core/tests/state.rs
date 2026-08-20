// SPDX-License-Identifier: AGPL-3.0

use std::fs;
use std::path::{Path, PathBuf};

use candi_core::{Error, Load, Position, load, save, sidecar_path};

fn temp_fixture_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "candi-state-test-{}-{}-{}",
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

fn pdf_path(dir: &Path, bytes: &[u8]) -> PathBuf {
    let path = dir.join("book.pdf");
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn sidecar_path_appends_suffix_in_same_directory() {
    let pdf = Path::new("/tmp/docs/book.pdf");
    assert_eq!(
        sidecar_path(pdf),
        PathBuf::from("/tmp/docs/book.pdf.candi.toml")
    );
}

#[test]
fn round_trip_save_and_load() {
    let dir = temp_fixture_dir("round-trip");
    let pdf = pdf_path(&dir, b"%PDF-1.4 dummy");
    let position = Position::new(42, 12, "2026-08-20T12:00:00Z");

    save(&pdf, &position).unwrap();

    match load(&pdf).unwrap() {
        Load::Loaded(loaded) => {
            assert_eq!(loaded.page(), 42);
            assert_eq!(loaded.scroll(), 12);
            assert!(
                loaded.updated_at().ends_with('Z'),
                "updated_at should be RFC3339 UTC: {}",
                loaded.updated_at()
            );
        }
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn load_known_timestamp_from_sidecar_file() {
    let dir = temp_fixture_dir("known-ts");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    let sidecar = sidecar_path(&pdf);
    fs::write(
        &sidecar,
        r#"schema_version = 1
[reading]
page = 7
scroll = 3
updated_at = "2026-08-20T12:00:00Z"
"#,
    )
    .unwrap();

    match load(&pdf).unwrap() {
        Load::Loaded(position) => {
            assert_eq!(position.page(), 7);
            assert_eq!(position.scroll(), 3);
            assert_eq!(position.updated_at(), "2026-08-20T12:00:00Z");
        }
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn missing_sidecar_returns_missing() {
    let dir = temp_fixture_dir("missing");
    let pdf = pdf_path(&dir, b"%PDF-1.4");

    assert!(matches!(load(&pdf).unwrap(), Load::Missing));
}

#[test]
fn corrupt_toml_returns_corrupt() {
    let dir = temp_fixture_dir("corrupt");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    fs::write(sidecar_path(&pdf), "not valid {{{ toml").unwrap();

    match load(&pdf).unwrap() {
        Load::Corrupt(message) => assert!(!message.is_empty()),
        other => panic!("expected Corrupt, got {other:?}"),
    }
}

#[test]
fn schema_version_two_returns_unsupported_schema() {
    let dir = temp_fixture_dir("schema-v2");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    fs::write(
        sidecar_path(&pdf),
        r#"schema_version = 2
[reading]
page = 0
scroll = 0
updated_at = "2026-08-20T12:00:00Z"
"#,
    )
    .unwrap();

    match load(&pdf) {
        Err(Error::UnsupportedSchema { found: 2 }) => {}
        other => panic!("expected UnsupportedSchema {{ found: 2 }}, got {other:?}"),
    }
}

#[test]
fn schema_version_zero_returns_corrupt() {
    let dir = temp_fixture_dir("schema-v0");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    fs::write(
        sidecar_path(&pdf),
        r#"schema_version = 0
[reading]
page = 0
scroll = 0
updated_at = "2026-08-20T12:00:00Z"
"#,
    )
    .unwrap();

    assert!(matches!(load(&pdf).unwrap(), Load::Corrupt(_)));
}

#[test]
fn empty_schema_version_returns_corrupt() {
    let dir = temp_fixture_dir("schema-empty");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    fs::write(
        sidecar_path(&pdf),
        r#"schema_version = ""
[reading]
page = 0
scroll = 0
updated_at = "2026-08-20T12:00:00Z"
"#,
    )
    .unwrap();

    assert!(matches!(load(&pdf).unwrap(), Load::Corrupt(_)));
}

#[test]
fn save_does_not_modify_pdf_bytes() {
    let dir = temp_fixture_dir("pdf-unchanged");
    let pdf_bytes = b"%PDF-1.7\n% unchanged payload\n";
    let pdf = pdf_path(&dir, pdf_bytes);
    let before = fs::read(&pdf).unwrap();

    save(&pdf, &Position::new(1, 2, "2026-08-20T12:00:00Z")).unwrap();

    assert_eq!(fs::read(&pdf).unwrap(), before);
    assert!(sidecar_path(&pdf).exists());
}

#[test]
fn save_leaves_no_temp_file_on_success() {
    let dir = temp_fixture_dir("no-tmp");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    save(&pdf, &Position::new(0, 0, "2026-08-20T12:00:00Z")).unwrap();

    for entry in fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.ends_with(".tmp"),
            "unexpected temp file left behind: {name}"
        );
    }
}

#[test]
fn failed_save_keeps_existing_sidecar_intact() {
    let dir = temp_fixture_dir("atomic-fail");
    let pdf = pdf_path(&dir, b"%PDF-1.4");
    let sidecar = sidecar_path(&pdf);
    let original = r#"schema_version = 1
[reading]
page = 9
scroll = 4
updated_at = "2026-08-20T12:00:00Z"
"#;
    fs::write(&sidecar, original).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();

        let err = save(&pdf, &Position::new(99, 99, "2026-08-20T12:00:00Z")).unwrap_err();
        assert!(matches!(err, Error::Io(_)));

        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(fs::read_to_string(&sidecar).unwrap(), original);
    }

    #[cfg(not(unix))]
    {
        eprintln!("skipped: atomic failure test requires unix");
    }
}
