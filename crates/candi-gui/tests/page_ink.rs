// SPDX-License-Identifier: AGPL-3.0

//! End-to-end ink pins: a MuPDF render pushed through theme recolor must
//! leave visible text on sparse pages. Guards two regressions that both
//! surfaced as blank pages: percentile collapse at 100% zoom and the same
//! wipe at fit-width scales (~550% on a 200 pt fixture).

use std::path::PathBuf;

use candi_pdf::{BackendKind, open};
use candi_theme::{Color, recolor};

fn tiny() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/candi-pdf/tests/fixtures/tiny.pdf")
        .to_str()
        .expect("fixture path")
        .to_owned()
}

/// Fraction of clearly-inked pixels (luma < 200) after recoloring with the
/// Light theme's page colors.
fn ink_fraction(scale: f32) -> f32 {
    let doc = open(BackendKind::Mupdf, &tiny(), None).expect("tiny.pdf opens");
    let img = doc.render_page(0, scale).expect("page renders");
    let mut rgba = img.rgba.clone();
    let bg = Color::from([0xFF, 0xFF, 0xFF, 0xFF]);
    let fg = Color::from([0x1A, 0x1A, 0x1A, 0xFF]);
    recolor(&mut rgba, bg, fg);
    let inked = rgba
        .chunks_exact(4)
        .filter(|px| {
            77 * u32::from(px[0]) + 151 * u32::from(px[1]) + 28 * u32::from(px[2]) < (200u32 << 8)
        })
        .count();
    inked as f32 / ((img.width * img.height) as f32)
}

#[test]
fn tiny_pdf_has_visible_text_at_100_percent() {
    assert!(
        ink_fraction(1.0) > 0.002,
        "single-line fixture must keep its text through render+recolor"
    );
}

#[test]
fn tiny_pdf_stays_visible_at_fit_width_scale() {
    assert!(
        ink_fraction(5.5) > 0.002,
        "high-zoom renders must not blank out"
    );
}
