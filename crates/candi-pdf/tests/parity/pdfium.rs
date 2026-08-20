// SPDX-License-Identifier: AGPL-3.0

use candi_pdf::{BackendKind, Document, Error, open};

#[path = "mod.rs"]
mod suite;

fn open_pdfium(path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error> {
    open(BackendKind::Pdfium, path, password)
}

#[test]
fn parity_suite_pdfium() {
    suite::run_suite(open_pdfium);
}
