// SPDX-License-Identifier: AGPL-3.0

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use candi_core::sidecar_path;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_candi"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../candi-pdf/tests/fixtures/{name}"))
}

fn bench_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../bench/fixtures/{name}"))
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

fn copy_fixture(src: &Path, dir: &Path, name: &str) -> PathBuf {
    let dest = dir.join(name);
    fs::copy(src, &dest).unwrap();
    dest
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn run_with_env(key: &str, value: &str, args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .env(key, value)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn no_args_exits_nonzero() {
    let output = run(&[]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn unknown_flag_exits_one() {
    let pdf = fixture("tiny.pdf");
    let path = pdf.to_str().unwrap();
    let output = run(&["--nope", path]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn missing_file_exits_with_not_found_message() {
    let output = run(&["/tmp/candi-cli-does-not-exist.pdf"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("file not found") || stderr.contains("not found"),
        "stderr: {stderr}"
    );
}

#[test]
fn unknown_backend_exits_with_unsupported_message() {
    let pdf = fixture("tiny.pdf");
    let path = pdf.to_str().unwrap();
    let output = run(&["--backend", "nope", path]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsupported"), "stderr: {stderr}");
}

#[test]
fn encrypted_pdf_exits_with_encrypted_message() {
    let pdf = bench_fixture("dummy-encrypted.pdf");
    let path = pdf.to_str().unwrap();
    let output = run(&[path]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("encrypted"), "stderr: {stderr}");
}

#[test]
fn image_only_pdf_exits_with_no_text_layer_message() {
    let dir = temp_dir("image-only");
    let pdf = copy_fixture(&fixture("image-only.pdf"), &dir, "image-only.pdf");
    let path = pdf.to_str().unwrap();
    let output = run(&[path]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("text layer") || stderr.contains("image-only"),
        "stderr: {stderr}"
    );
}

#[test]
fn headless_tiny_pdf_prints_first_page() {
    let dir = temp_dir("headless-tiny");
    let pdf = copy_fixture(&fixture("tiny.pdf"), &dir, "tiny.pdf");
    let path = pdf.to_str().unwrap();
    let output = run_with_env("CANDI_NO_TUI", "1", &[path]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "page=1");
    assert!(sidecar_path(&pdf).exists());
    let sidecar = fs::read_to_string(sidecar_path(&pdf)).unwrap();
    assert!(sidecar.contains("schema_version = 1"));
}

#[test]
fn unsupported_sidecar_schema_exits_with_message() {
    let dir = temp_dir("unsupported-schema");
    let pdf = copy_fixture(&fixture("tiny.pdf"), &dir, "tiny.pdf");
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

    let path = pdf.to_str().unwrap();
    let output = run_with_env("CANDI_NO_TUI", "1", &[path]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported sidecar schema version"),
        "stderr: {stderr}"
    );
}

#[test]
fn headless_resumes_saved_page() {
    let dir = temp_dir("resume");
    let pdf = copy_fixture(&fixture("blank-first-page.pdf"), &dir, "book.pdf");
    fs::write(
        sidecar_path(&pdf),
        r#"schema_version = 1
[reading]
page = 1
scroll = 0
updated_at = "2026-08-20T12:00:00Z"
"#,
    )
    .unwrap();

    let path = pdf.to_str().unwrap();
    let output = run_with_env("CANDI_NO_TUI", "1", &[path]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "page=2");
}

#[test]
fn corrupt_sidecar_warns_and_starts_fresh() {
    let dir = temp_dir("corrupt-sidecar");
    let pdf = copy_fixture(&fixture("tiny.pdf"), &dir, "tiny.pdf");
    fs::write(sidecar_path(&pdf), "not valid {{{ toml").unwrap();

    let path = pdf.to_str().unwrap();
    let output = run_with_env("CANDI_NO_TUI", "1", &[path]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "page=1");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning") && stderr.contains("corrupt"),
        "stderr: {stderr}"
    );
}
