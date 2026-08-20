// SPDX-License-Identifier: AGPL-3.0

//! Committed fixture paths and runtime generation helpers for integration tests.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn tiny() -> PathBuf {
    manifest_dir().join("tests/fixtures/tiny.pdf")
}

pub fn zero_pages() -> PathBuf {
    manifest_dir().join("tests/fixtures/zero-pages.pdf")
}

pub fn blank_first_page() -> PathBuf {
    manifest_dir().join("tests/fixtures/blank-first-page.pdf")
}

pub fn image_only() -> PathBuf {
    manifest_dir().join("tests/fixtures/image-only.pdf")
}

pub fn encrypted() -> PathBuf {
    manifest_dir().join("../../bench/fixtures/dummy-encrypted.pdf")
}

pub fn write_truncated_tiny(dest: &Path) {
    let tiny = fs::read(tiny()).expect("tiny.pdf fixture");
    let len = 100.min(tiny.len());
    fs::write(dest, &tiny[..len]).expect("write truncated PDF");
}

/// Generate a single-page image-only PDF at `dest`.
///
/// Hard-fails when `magick` (ImageMagick) is not installed. Intended for local
/// one-off fixture generation; CI uses the committed `image-only.pdf`.
#[allow(dead_code)]
pub fn generate_image_only_pdf(dest: &Path) {
    let magick = which_magick().expect(
        "image-only fixture generation requires ImageMagick (`magick` or `convert` on PATH)",
    );

    let dir = env::temp_dir();
    let png = dir.join("candi-image-only-src.png");
    let status = Command::new(&magick)
        .args(["-size", "100x50", "xc:white", png.to_str().unwrap()])
        .status()
        .unwrap_or_else(|err| {
            panic!("failed to run {magick} to create source PNG: {err}");
        });
    if !status.success() {
        panic!("{magick} failed to create source PNG (exit {status})");
    }

    let status = Command::new(&magick)
        .args([png.to_str().unwrap(), dest.to_str().unwrap()])
        .status()
        .unwrap_or_else(|err| {
            panic!("failed to run {magick} to create image-only PDF: {err}");
        });
    if !status.success() {
        panic!("{magick} failed to create image-only PDF (exit {status})");
    }

    let _ = fs::remove_file(png);
}

fn which_magick() -> Option<String> {
    for candidate in ["magick", "convert"] {
        if Command::new(candidate)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(candidate.into());
        }
    }
    None
}
