// SPDX-License-Identifier: AGPL-3.0

//! Trait-contract integration tests: the crate as seen by an external
//! consumer — errors, factory selection, and the `Document`/`Backend`
//! contracts through public APIs and trait objects.

use candi_pdf::stub::{StubBackend, StubPage};
use candi_pdf::{
    Backend, BackendKind, Block, Document, Error, Line, PagePositions, Word, available, open,
    open_default,
};

fn error_kinds() -> Vec<Error> {
    vec![
        Error::NotFound("missing.pdf".into()),
        Error::PermissionDenied("no read access".into()),
        Error::Encrypted("needs password".into()),
        Error::WrongPassword("rejected".into()),
        Error::NoTextLayer,
        Error::Malformed("garbage bytes".into()),
        Error::Unsupported("unknown backend".into()),
        Error::Other("mystery failure".into()),
    ]
}

#[test]
fn error_display_is_human_readable_for_every_kind() {
    let cases = [
        (
            Error::NotFound("missing.pdf".into()),
            "file not found: missing.pdf",
        ),
        (
            Error::PermissionDenied("no read access".into()),
            "permission denied: no read access",
        ),
        (
            Error::Encrypted("needs password".into()),
            "encrypted document: needs password",
        ),
        (
            Error::WrongPassword("rejected".into()),
            "wrong password: rejected",
        ),
        (
            Error::NoTextLayer,
            "no extractable text layer (image-only or scanned)",
        ),
        (
            Error::Malformed("garbage bytes".into()),
            "malformed document: garbage bytes",
        ),
        (
            Error::Unsupported("unknown backend".into()),
            "unsupported: unknown backend",
        ),
        (Error::Other("mystery failure".into()), "mystery failure"),
    ];
    for (err, expected) in cases {
        assert_eq!(err.to_string(), expected);
    }
}

#[test]
fn error_implements_std_error() {
    fn assert_std_error<T: std::error::Error>() {}
    assert_std_error::<Error>();
}

#[test]
fn backend_name_via_trait_object() {
    let backend: Box<dyn Backend> = Box::new(StubBackend::new(0));
    assert_eq!(backend.name(), "stub");
}

#[test]
fn document_trait_object_contract() {
    let mut backend = StubBackend::new(3);
    backend.pages[0].text = Ok("cover page, legitimately empty".into());
    backend.pages[1].text = Ok("first real page".into());
    backend.pages[2].positions = Err(Error::PermissionDenied("geo blocked".into()));

    let doc: Box<dyn Document> = backend.open("book.pdf", Some("pw")).unwrap();

    // page_count: cached at open, infallible, stable across calls.
    assert_eq!(doc.page_count(), 3);
    assert_eq!(doc.page_count(), 3);

    // page_text: lazy, one page at a time — requesting page 2 first does not
    // disturb pages 0/1, and out-of-range is a kinded error.
    assert_eq!(doc.page_text(2).unwrap(), "");
    assert_eq!(doc.page_text(1).unwrap(), "first real page");
    assert_eq!(doc.page_text(0).unwrap(), "cover page, legitimately empty");
    assert!(matches!(doc.page_text(3), Err(Error::Other(_))));

    // positions: Ok(None) models no positional API; real errors propagate.
    assert_eq!(doc.page_positions(0).unwrap(), None);
    assert_eq!(
        doc.page_positions(2).unwrap_err(),
        Error::PermissionDenied("geo blocked".into())
    );
}

#[test]
fn scripted_error_kinds_flow_through_the_public_contract_verbatim() {
    for kind in error_kinds() {
        let mut backend = StubBackend::new(1);
        backend.pages[0].text = Err(kind.clone());
        let doc: Box<dyn Document> = backend.open("book.pdf", None).unwrap();
        assert_eq!(doc.page_text(0).unwrap_err(), kind);
    }
}

#[test]
fn empty_page_returns_empty_string() {
    let doc = StubBackend::new(1).open("book.pdf", None).unwrap();
    assert_eq!(doc.page_text(0).unwrap(), "");
}

#[test]
fn positions_ok_some_round_trips_through_public_api() {
    let mut backend = StubBackend::new(1);
    backend.pages[0].positions = Ok(Some(PagePositions {
        blocks: vec![Block {
            lines: vec![Line {
                words: vec![Word {
                    text: "word".into(),
                    x: 0.5,
                    y: 1.5,
                    font_size: 10.0,
                }],
            }],
        }],
    }));
    let doc = backend.open("book.pdf", None).unwrap();
    let positions = doc.page_positions(0).unwrap().unwrap();
    assert_eq!(positions.blocks[0].lines[0].words[0].text, "word");
}

#[cfg(all(feature = "mupdf-backend", feature = "pdfium-backend"))]
#[test]
fn available_lists_both_backends_in_order() {
    assert_eq!(available(), vec!["mupdf", "pdfium"]);
}

#[cfg(all(feature = "mupdf-backend", not(feature = "pdfium-backend")))]
#[test]
fn available_lists_only_mupdf() {
    assert_eq!(available(), vec!["mupdf"]);
}

#[cfg(all(not(feature = "mupdf-backend"), feature = "pdfium-backend"))]
#[test]
fn available_lists_only_pdfium() {
    assert_eq!(available(), vec!["pdfium"]);
}

#[cfg(feature = "mupdf-backend")]
#[test]
fn open_mupdf_routes_to_real_backend() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.pdf");
    assert!(open(BackendKind::Mupdf, path.to_str().unwrap(), None).is_ok());
}

#[cfg(feature = "mupdf-backend")]
#[test]
fn open_default_routes_to_mupdf() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.pdf");
    assert!(open_default(path.to_str().unwrap(), None).is_ok());
}

#[cfg(not(feature = "mupdf-backend"))]
#[test]
fn open_mupdf_is_unsupported_without_feature() {
    assert!(matches!(
        open(BackendKind::Mupdf, "unused.pdf", None),
        Err(Error::Unsupported(_))
    ));
}

#[cfg(feature = "pdfium-backend")]
#[test]
fn open_pdfium_returns_a_document() {
    let doc = open(BackendKind::Pdfium, "unused.pdf", None).unwrap();
    assert_eq!(doc.page_count(), 1);
    assert_eq!(doc.page_text(0).unwrap(), "");
}

#[cfg(not(feature = "pdfium-backend"))]
#[test]
fn open_pdfium_is_unsupported_without_feature() {
    assert!(matches!(
        open(BackendKind::Pdfium, "unused.pdf", None),
        Err(Error::Unsupported(_))
    ));
}

#[cfg(not(feature = "mupdf-backend"))]
#[test]
fn open_default_is_unsupported_without_mupdf() {
    assert!(matches!(
        open_default("unused.pdf", None),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn stub_scripting_is_fully_configurable_through_public_fields() {
    let mut backend = StubBackend::new(2);
    backend.pages[1] = StubPage {
        text: Ok("configured".into()),
        positions: Ok(None),
    };
    let doc = backend.open("book.pdf", None).unwrap();
    assert_eq!(doc.page_text(1).unwrap(), "configured");
}
