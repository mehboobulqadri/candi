// SPDX-License-Identifier: AGPL-3.0

//! Candi CLI: open a PDF, resume reading position, and launch the TUI.
//!
//! Set `CANDI_NO_TUI=1` to skip the interactive UI (prints `page=<n>` and saves position).

use std::path::Path;
use std::process::ExitCode;

use candi_core::{Load, Position, ViewState, load, save};
use candi_pdf::{BackendKind, Error as PdfError, open};
use clap::Parser;

/// Scroll cap before TUI resize; page max scroll is unknown until then.
const UNBOUND_SCROLL: usize = usize::MAX;

#[derive(Parser)]
#[command(
    name = "candi",
    about = "Keyboard-first PDF reader",
    disable_version_flag = true
)]
struct Args {
    /// PDF file to open
    file: String,

    /// Document backend (`mupdf` or `pdfium`)
    #[arg(long, default_value = "mupdf")]
    backend: String,
}

fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(err) => {
            let _ = err.print();
            return ExitCode::from(1);
        }
    };
    let path = Path::new(&args.file);

    let kind = match args.backend.as_str() {
        "mupdf" => BackendKind::Mupdf,
        "pdfium" => BackendKind::Pdfium,
        other => {
            eprintln!("{}", PdfError::Unsupported(format!("backend '{other}'")));
            return ExitCode::from(1);
        }
    };

    let pdf = match open(kind, &args.file, None) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    let view = match load(path) {
        Ok(load) => view_from_load(pdf.as_ref(), load),
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    if std::env::var("CANDI_NO_TUI").as_deref() == Ok("1") {
        println!("page={}", view.page() + 1);
        if let Err(err) = save_position(path, view) {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    let view = match candi_tui::run(pdf, &args.file, view) {
        Ok(view) => view,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    if let Err(err) = save_position(path, view) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn view_from_load(pdf: &dyn candi_pdf::Document, load: Load) -> ViewState {
    match load {
        Load::Missing => ViewState::new(),
        Load::Loaded(position) => ViewState::new()
            .goto_page(position.page(), pdf.page_count())
            .scroll_down(position.scroll(), UNBOUND_SCROLL),
        Load::Corrupt(message) => {
            eprintln!("warning: corrupt sidecar: {message}");
            ViewState::new()
        }
    }
}

fn save_position(path: &Path, view: ViewState) -> Result<(), candi_core::Error> {
    save(path, &Position::new(view.page(), view.scroll_offset(), ""))
}
