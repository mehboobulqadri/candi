// SPDX-License-Identifier: AGPL-3.0

use candi_pdf::{BackendKind, Document, Error, open};

#[path = "mod.rs"]
mod suite;

fn open_mupdf(path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error> {
    open(BackendKind::Mupdf, path, password)
}

#[test]
fn parity_suite_mupdf() {
    suite::run_suite(open_mupdf);
}
