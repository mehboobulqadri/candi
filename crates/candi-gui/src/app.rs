// SPDX-License-Identifier: AGPL-3.0

use std::path::PathBuf;

use candi_cli::{UNBOUND_SCROLL, open_document, save_reading_position};
use candi_core::{SearchSession, ViewState, normalize_reader_text};
use candi_pdf::{BackendKind, Document, Error as PdfError};
use eframe::egui;

struct ActiveSearch {
    results: Vec<(usize, usize)>,
    index: Option<usize>,
}

pub struct ReaderApp {
    backend: BackendKind,
    path: Option<PathBuf>,
    filename: String,
    document: Option<Box<dyn Document>>,
    view: ViewState,
    page_text: String,
    error: Option<String>,
    search_query: String,
    search: Option<ActiveSearch>,
}

impl ReaderApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        initial: Option<PathBuf>,
        backend: BackendKind,
    ) -> Self {
        let mut app = Self {
            backend,
            path: None,
            filename: String::new(),
            document: None,
            view: ViewState::new(),
            page_text: String::new(),
            error: None,
            search_query: String::new(),
            search: None,
        };
        if let Some(path) = initial {
            app.open_path(path);
        }
        app
    }

    fn open_path(&mut self, path: PathBuf) {
        self.error = None;
        self.search = None;
        self.search_query.clear();
        match open_document(&path, self.backend) {
            Ok(opened) => {
                self.path = Some(path.clone());
                self.filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("document.pdf")
                    .to_owned();
                self.view = opened.view;
                self.document = Some(opened.document);
                if let Err(err) = self.reload_page() {
                    self.show_error(err.to_string());
                }
            }
            Err(err) => {
                self.document = None;
                self.path = Some(path);
                self.page_text.clear();
                self.error = Some(err.to_string());
            }
        }
    }

    fn reload_page(&mut self) -> Result<(), PdfError> {
        let Some(document) = self.document.as_ref() else {
            self.page_text.clear();
            return Ok(());
        };
        let page_count = document.page_count();
        if page_count == 0 {
            self.page_text.clear();
            return Err(PdfError::Malformed("document has no pages".into()));
        }
        let page = self.view.page().min(page_count - 1);
        let scroll = self.view.scroll_offset();
        self.view = self
            .view
            .goto_page(page, page_count)
            .scroll_down(scroll, UNBOUND_SCROLL);
        let text = document.page_text(page)?;
        self.page_text = normalize_reader_text(&text);
        Ok(())
    }

    fn page_count(&self) -> usize {
        self.document
            .as_ref()
            .map(|doc| doc.page_count())
            .unwrap_or(0)
    }

    fn show_error(&mut self, message: String) {
        self.error = Some(message);
    }

    fn save_position(&self) {
        let (Some(path), Some(_)) = (&self.path, &self.document) else {
            return;
        };
        if let Err(err) = save_reading_position(path, self.view) {
            eprintln!("{err}");
        }
    }

    fn set_scroll_lines(&mut self, lines: usize) {
        let current = self.view.scroll_offset();
        if lines > current {
            self.view = self.view.scroll_down(lines - current, UNBOUND_SCROLL);
        } else {
            self.view = self.view.scroll_up(current - lines, UNBOUND_SCROLL);
        }
    }

    fn start_search(&mut self) {
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let query = self.search_query.trim();
        if query.is_empty() {
            self.search = None;
            return;
        }
        match collect_search(document.as_ref(), query, self.view.page()) {
            Ok(results) => {
                self.search = Some(ActiveSearch {
                    index: if results.is_empty() { None } else { Some(0) },
                    results,
                });
                self.show_current_match();
            }
            Err(err) => self.show_error(err.to_string()),
        }
    }

    fn jump_search_next(&mut self) {
        let Some(search) = self.search.as_mut() else {
            return;
        };
        if search.results.is_empty() {
            return;
        }
        let idx = search.index.unwrap_or(0);
        search.index = Some((idx + 1) % search.results.len());
        self.show_current_match();
    }

    fn jump_search_prev(&mut self) {
        let Some(search) = self.search.as_mut() else {
            return;
        };
        if search.results.is_empty() {
            return;
        }
        let idx = search.index.unwrap_or(0);
        search.index = Some((idx + search.results.len() - 1) % search.results.len());
        self.show_current_match();
    }

    fn show_current_match(&mut self) {
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let Some(search) = self.search.as_ref() else {
            return;
        };
        let Some(idx) = search.index else {
            return;
        };
        let Some(&(page, offset)) = search.results.get(idx) else {
            return;
        };
        let page_count = document.page_count();
        self.view = self.view.goto_page(page, page_count);
        if let Err(err) = self.reload_page() {
            self.show_error(err.to_string());
            return;
        }
        self.set_scroll_lines(line_for_byte_offset(&self.page_text, offset));
    }

    fn search_status(&self) -> Option<String> {
        let search = self.search.as_ref()?;
        let total = search.results.len();
        if total == 0 {
            return Some("no matches".into());
        }
        let current = search.index.map(|idx| idx + 1).unwrap_or(0);
        Some(format!("{current}/{total}"))
    }

    fn draw_page_text(&mut self, ui: &mut egui::Ui) {
        let line_height = ui.text_style_height(&egui::TextStyle::Body);
        let scroll_y = self.view.scroll_offset() as f32 * line_height;
        let output = egui::ScrollArea::vertical()
            .id_salt("page_text")
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .vertical_scroll_offset(scroll_y)
            .show(ui, |ui| {
                let mut text = self.page_text.as_str();
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .desired_width(f32::INFINITY)
                        .interactive(false)
                        .frame(false),
                );
            });
        let lines = (output.state.offset.y / line_height).round() as usize;
        if lines != self.view.scroll_offset() {
            self.set_scroll_lines(lines);
        }
    }
}

impl eframe::App for ReaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Open").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .pick_file()
                {
                    self.open_path(path);
                }
                if ui.button("Quit").clicked() {
                    self.save_position();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.page_count() > 0 {
                    ui.label(format!(
                        "page {}/{}",
                        self.view.page() + 1,
                        self.page_count()
                    ));
                }
                if !self.filename.is_empty() {
                    ui.separator();
                    ui.label(&self.filename);
                }
                if let Some(status) = self.search_status() {
                    ui.separator();
                    ui.label(status);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut self.search_query);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.start_search();
                }
                if ui.button("Find").clicked() {
                    self.start_search();
                }
                if ui.button("Next").clicked() {
                    self.jump_search_next();
                }
                if ui.button("Prev").clicked() {
                    self.jump_search_prev();
                }
            });

            ui.horizontal(|ui| {
                let prev = ui.add_enabled(self.view.page() > 0, egui::Button::new("Prev page"));
                let next = ui.add_enabled(
                    self.page_count() > 0 && self.view.page() + 1 < self.page_count(),
                    egui::Button::new("Next page"),
                );
                if prev.clicked() {
                    self.view = self.view.prev_page(self.page_count());
                    if let Err(err) = self.reload_page() {
                        self.show_error(err.to_string());
                    }
                }
                if next.clicked() {
                    self.view = self.view.next_page(self.page_count());
                    if let Err(err) = self.reload_page() {
                        self.show_error(err.to_string());
                    }
                }
            });

            if self.document.is_some() && self.error.is_none() {
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| self.draw_page_text(ui),
                );
            } else if self.document.is_none() && self.error.is_none() {
                ui.label("Open a PDF from the menu or pass a path on the command line.");
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_position();
    }
}

fn collect_search(
    document: &dyn Document,
    query: &str,
    start_page: usize,
) -> Result<Vec<(usize, usize)>, PdfError> {
    let mut session = SearchSession::new(document, query, start_page);
    let mut results = Vec::new();
    while let Some(hit) = session.next()? {
        if results.contains(&hit) {
            break;
        }
        results.push(hit);
    }
    Ok(results)
}

fn line_for_byte_offset(text: &str, offset: usize) -> usize {
    text.bytes()
        .take(offset.min(text.len()))
        .filter(|&byte| byte == b'\n')
        .count()
}
