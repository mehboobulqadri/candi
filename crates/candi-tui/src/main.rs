// SPDX-License-Identifier: AGPL-3.0

use std::process;

use candi_pdf::open_default;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: candi-tui <pdf>");
        process::exit(1);
    };

    let document = match open_default(&path, None) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    if let Err(err) = candi_tui::run(document, &path) {
        eprintln!("{err}");
        process::exit(1);
    }
}
