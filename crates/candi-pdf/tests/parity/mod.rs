// SPDX-License-Identifier: AGPL-3.0

//! Shared backend parity suite — identical kind assertions for every engine.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use candi_pdf::{Document, Error};
#[cfg(any(not(feature = "mupdf-backend"), not(feature = "pdfium-backend")))]
use candi_pdf::BackendKind;

#[path = "../common/fixtures.rs"]
mod fixtures;

pub type OpenFn = fn(&str, Option<&str>) -> Result<Box<dyn Document>, Error>;

const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

pub fn run_suite(open_fn: OpenFn) {
    hardening_not_found(open_fn);
    hardening_permission_denied(open_fn);
    hardening_truncated_malformed(open_fn);
    hardening_encrypted_no_password(open_fn);
    hardening_wrong_password(open_fn);
    hardening_image_only_no_text_layer(open_fn);
    hardening_unsupported(open_fn);
    hardening_zero_pages_malformed(open_fn);
    parity_tiny_opens_with_text(open_fn);
    parity_blank_first_page_not_no_text_layer(open_fn);
    parity_positions_sanity(open_fn);
}

fn with_open_timeout<F>(label: &str, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    match rx.recv_timeout(OPEN_TIMEOUT) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("{label}: open hung past {}s", OPEN_TIMEOUT.as_secs());
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("{label}: open thread disconnected before completing");
        }
    }
}

pub fn hardening_not_found(open_fn: OpenFn) {
    with_open_timeout("not_found", move || {
        assert!(matches!(
            open_fn("/no/such/candi-parity-fixture.pdf", None),
            Err(Error::NotFound(_))
        ));
    });
}

pub fn hardening_permission_denied(open_fn: OpenFn) {
    #[cfg(not(unix))]
    {
        eprintln!("SKIP permission_denied: unix-only chmod 000");
        let _ = open_fn;
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let tiny = fixtures::tiny();
        let path = env_temp_path("candi-parity-unreadable.pdf");
        let _ = fs::remove_file(&path);
        fs::copy(&tiny, &path).expect("copy tiny.pdf for permission test");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))
            .expect("chmod 000");

        if fs::read(&path).is_ok() {
            eprintln!("SKIP permission_denied: chmod 000 ineffective (likely root)");
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
            let _ = fs::remove_file(path);
            return;
        }

        let path_str = path.to_str().unwrap().to_string();
        with_open_timeout("permission_denied", move || {
            assert!(matches!(
                open_fn(&path_str, None),
                Err(Error::PermissionDenied(_))
            ));
        });
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
        let _ = fs::remove_file(path);
    }
}

pub fn hardening_truncated_malformed(open_fn: OpenFn) {
    let path = env_temp_path("candi-parity-truncated.pdf");
    fixtures::write_truncated_tiny(&path);
    let path_str = path.to_str().unwrap().to_string();
    with_open_timeout("truncated_malformed", move || {
        assert!(matches!(open_fn(&path_str, None), Err(Error::Malformed(_))));
    });
    let _ = fs::remove_file(path);
}

pub fn hardening_encrypted_no_password(open_fn: OpenFn) {
    let path = fixtures::encrypted().to_str().unwrap().to_string();
    with_open_timeout("encrypted_no_password", move || {
        assert!(matches!(open_fn(&path, None), Err(Error::Encrypted(_))));
    });
}

pub fn hardening_wrong_password(open_fn: OpenFn) {
    let path = fixtures::encrypted().to_str().unwrap().to_string();
    with_open_timeout("wrong_password", move || {
        assert!(matches!(
            open_fn(&path, Some("bad")),
            Err(Error::WrongPassword(_))
        ));
    });
}

pub fn hardening_image_only_no_text_layer(open_fn: OpenFn) {
    let path = fixtures::image_only();
    assert!(
        path.exists(),
        "committed image-only fixture missing at {}",
        path.display()
    );
    let path_str = path.to_str().unwrap().to_string();
    with_open_timeout("image_only_no_text_layer", move || {
        assert!(matches!(open_fn(&path_str, None), Err(Error::NoTextLayer)));
    });
}

/// PDF-feature `Unsupported` is not pinned until a dual-engine fixture exists (01/04 risk).
/// Factory gating for uncompiled backends is the honest automated coverage for that row.
pub fn hardening_unsupported(_: OpenFn) {
    #[cfg(not(feature = "mupdf-backend"))]
    with_open_timeout("unsupported_mupdf_gated", || {
        assert!(matches!(
            candi_pdf::open(BackendKind::Mupdf, "unused.pdf", None),
            Err(Error::Unsupported(_))
        ));
    });

    #[cfg(not(feature = "pdfium-backend"))]
    with_open_timeout("unsupported_pdfium_gated", || {
        assert!(matches!(
            candi_pdf::open(BackendKind::Pdfium, "unused.pdf", None),
            Err(Error::Unsupported(_))
        ));
    });
}

pub fn hardening_zero_pages_malformed(open_fn: OpenFn) {
    let path = fixtures::zero_pages().to_str().unwrap().to_string();
    with_open_timeout("zero_pages_malformed", move || {
        assert!(matches!(open_fn(&path, None), Err(Error::Malformed(_))));
    });
}

pub fn parity_tiny_opens_with_text(open_fn: OpenFn) {
    let path = fixtures::tiny().to_str().unwrap().to_string();
    with_open_timeout("tiny_opens_with_text", move || {
        let doc = open_fn(&path, None).expect("tiny.pdf should open");
        assert_eq!(doc.page_count(), 1);
        assert!(doc.page_text(0).unwrap().contains("Hello Candi"));
    });
}

pub fn parity_blank_first_page_not_no_text_layer(open_fn: OpenFn) {
    let path = fixtures::blank_first_page().to_str().unwrap().to_string();
    with_open_timeout("blank_first_page_not_no_text_layer", move || {
        let doc = open_fn(&path, None).expect("blank-first-page.pdf should open");
        assert_eq!(doc.page_count(), 2);
        assert_eq!(doc.page_text(0).unwrap(), "");
        assert!(doc.page_text(1).unwrap().contains("Page two"));
    });
}

pub fn parity_positions_sanity(open_fn: OpenFn) {
    let path = fixtures::tiny().to_str().unwrap().to_string();
    with_open_timeout("positions_sanity", move || {
        let doc = open_fn(&path, None).expect("tiny.pdf should open");
        let positions = doc
            .page_positions(0)
            .expect("positions call")
            .expect("backend exposes positions");
        assert!(!positions.blocks.is_empty());
        assert!(!positions.blocks[0].lines.is_empty());
        assert!(!positions.blocks[0].lines[0].words.is_empty());
    });
}

fn env_temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}
