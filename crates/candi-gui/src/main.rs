// SPDX-License-Identifier: AGPL-3.0

mod app;
mod highlight;
mod icons;
mod keybinds;
mod render;
mod sidebar;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use candi_cli::{open_document, parse_backend, save_reading_position};
use clap::Parser;
use eframe::egui;

use app::ReaderApp;

#[derive(Parser)]
#[command(
    name = "candi",
    about = "Candi PDF reader",
    disable_version_flag = true
)]
struct Args {
    /// PDF file to open
    file: Option<String>,

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

    if std::env::var("CANDI_NO_GUI").as_deref() == Ok("1") {
        return run_headless(args);
    }

    let initial = args.file.as_deref().map(PathBuf::from).or_else(pick_pdf);
    let backend = match parse_backend(&args.backend) {
        Ok(kind) => kind,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("candi")
            .with_decorations(false)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_icon(window_icon()),
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "Candi",
        options,
        Box::new(move |cc| Ok(Box::new(ReaderApp::new(cc, initial, backend)))),
    ) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn run_headless(args: Args) -> ExitCode {
    let Some(file) = args.file else {
        eprintln!("missing PDF path");
        return ExitCode::from(1);
    };
    let path = Path::new(&file);
    let kind = match parse_backend(&args.backend) {
        Ok(kind) => kind,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    let opened = match open_document(path, kind) {
        Ok(opened) => opened,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    println!("page={}", opened.view.page() + 1);
    if let Err(err) = save_reading_position(path, opened.view) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn window_icon() -> egui::IconData {
    let png = include_bytes!("../assets/icon-256.png");
    let img = image::load_from_memory(png).expect("bundled assets/icon-256.png is a valid PNG");
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    egui::IconData {
        width,
        height,
        rgba: rgba.into_raw(),
    }
}

fn pick_pdf() -> Option<PathBuf> {
    pdf_dialog().pick_file()
}

/// The single PDF file-picker construction shared by all open entry points.
pub(crate) fn pdf_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("PDF", &["pdf"])
}
