// SPDX-License-Identifier: AGPL-3.0

//! GUI reader shell: continuous page canvas over a background render
//! pipeline, chrome (top bar / sidebar / bottom bar), built-in theming, and
//! session persistence via `candi-cli`.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use candi_cli::{open_session, save_session};
use candi_core::{SearchSession, SessionState, ZoomMode};
use candi_pdf::{BackendKind, Document, Error as PdfError, PageImage};
use candi_theme::{BUILTIN_NAMES, Color, Theme, builtin, recolor};
use eframe::egui;
use egui::Key;

use crate::render::cache::{CacheKey, ImageCache};
use crate::render::layout::{self, GAP, Layout};
use crate::render::pipeline::{Pipeline, RenderRequest, RenderResult};

/// Pages kept as textures around the current page; texture memory outside this
/// window is released while the original bitmaps stay cached.
const TEXTURE_KEEP_AROUND: usize = 6;
const SIDEBAR_WIDTH: f32 = 260.0;
const SEARCH_FIELD_WIDTH: f32 = 180.0;

/// A promoted GPU texture for one page plus the scale it was rendered at.
struct PageTexture {
    scale_q: u16,
    handle: egui::TextureHandle,
}

#[derive(Clone, Copy, PartialEq)]
enum SidebarSection {
    Contents,
    Bookmarks,
    Search,
}

/// Keyboard shortcuts sampled once per frame.
#[derive(Clone, Copy)]
struct Shortcuts {
    open: bool,
    save: bool,
    sidebar: bool,
    find: bool,
    zoom_in: bool,
    zoom_out: bool,
    fit_width: bool,
    cycle_theme: bool,
    quit: bool,
    escape: bool,
    prev_page: bool,
    next_page: bool,
}

pub struct ReaderApp {
    backend: BackendKind,
    path: Option<PathBuf>,
    filename: String,
    document: Option<Arc<dyn Document>>,
    session: SessionState,
    theme: Theme,
    /// Name of the theme whose visuals were last pushed into egui.
    applied_theme: String,

    cache: ImageCache,
    pipeline: Option<Pipeline>,
    /// Render jobs queued on the worker, awaiting their result.
    pending: HashSet<CacheKey>,
    /// Render jobs that already failed at `failed_scale_q`; skipped until the
    /// zoom or document changes so a bad page cannot retry forever.
    failed: HashSet<CacheKey>,
    failed_scale_q: u16,
    /// Promoted textures keyed by page; dropped on theme switches.
    textures: HashMap<usize, PageTexture>,

    layout: Layout,
    /// `(avail_width, zoom)` the current layout was built for.
    layout_key: Option<(f32, ZoomMode)>,
    /// Per-page `(width, height)` in points, fetched once at open.
    page_sizes: Vec<(f32, f32)>,
    /// Effective quantized zoom percent; resolved from fit-width on relayout.
    zoom_pct: u16,

    sidebar_open: bool,
    section: SidebarSection,
    search_open: bool,
    focus_search: bool,
    search_query: String,
    search_hits: Option<Vec<usize>>,
    about_open: bool,

    /// Saved scroll fraction waiting for the first layout to apply.
    restore_frac: Option<f64>,
    /// Absolute content-y to scroll to on the next frame.
    pending_scroll: Option<f32>,
    primed: bool,
    error: Option<String>,
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
            session: SessionState::new(1),
            theme: builtin("Light").expect("built-in Light parses"),
            applied_theme: String::new(),
            cache: ImageCache::new(ImageCache::budget_from_env()),
            pipeline: None,
            pending: HashSet::new(),
            failed: HashSet::new(),
            failed_scale_q: 0,
            textures: HashMap::new(),
            layout: Layout::default(),
            layout_key: None,
            page_sizes: Vec::new(),
            zoom_pct: layout::MIN_ZOOM_PERCENT,
            sidebar_open: false,
            section: SidebarSection::Contents,
            search_open: false,
            focus_search: false,
            search_query: String::new(),
            search_hits: None,
            about_open: false,
            restore_frac: None,
            pending_scroll: None,
            primed: true,
            error: None,
        };
        if let Some(path) = initial {
            app.open_path(path);
        }
        app
    }

    fn open_path(&mut self, path: PathBuf) {
        self.error = None;
        self.search_hits = None;
        self.search_query.clear();
        match open_session(&path, self.backend) {
            Ok(opened) => {
                self.cache = ImageCache::new(ImageCache::budget_from_env());
                self.pending.clear();
                self.failed.clear();
                self.failed_scale_q = 0;
                self.textures.clear();
                self.pipeline = None;
                self.layout_key = None;
                self.page_sizes.clear();
                self.pending_scroll = None;
                self.primed = false;

                let theme_name = opened.session.theme.clone();
                self.session = opened.session;
                self.set_theme(&theme_name);
                self.path = Some(path.clone());
                self.filename = filename_of(&path);
                let document: Arc<dyn Document> = Arc::from(opened.document);
                self.pipeline = Some(Pipeline::spawn(document.clone()));
                self.document = Some(document);
                if let ZoomMode::Percent(p) = self.session.zoom {
                    self.zoom_pct = p;
                }
                self.restore_frac = Some(self.session.scroll_frac);
                self.load_page_sizes();
            }
            Err(err) => {
                self.error = Some(err.to_string());
            }
        }
    }

    /// Switch the active built-in theme. Visuals are re-applied at the top of
    /// the next [`ReaderApp::update`]; texture slots are dropped so pages
    /// re-promote from their cached originals in the new colors.
    fn set_theme(&mut self, name: &str) {
        self.theme =
            builtin(name).unwrap_or_else(|| builtin("Light").expect("built-in Light parses"));
        self.session.theme = self.theme.name.clone();
        self.textures.clear();
    }

    fn page_count(&self) -> usize {
        self.document.as_ref().map_or(0, |doc| doc.page_count())
    }

    fn goto_page(&mut self, page: usize) {
        let count = self.page_count();
        if count == 0 {
            return;
        }
        let page = page.min(count - 1);
        self.session.page = page;
        if let Some(rect) = self.layout.rects.get(page) {
            self.pending_scroll = Some((rect.y - GAP).max(0.0));
        }
    }

    fn zoom_step(&mut self, delta_percent: i16) {
        let pct = i32::from(self.zoom_pct) + i32::from(delta_percent);
        self.session.zoom = ZoomMode::Percent(layout::quantize_nearest(pct as f32));
    }

    fn save_state(&mut self) {
        let Some(path) = self.path.as_deref() else {
            return;
        };
        if let Err(err) = save_session(path, &self.session) {
            self.error = Some(format!("saving state: {err}"));
        }
    }

    fn open_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file()
        {
            self.open_path(path);
        }
    }

    fn run_search(&mut self) {
        let query = self.search_query.trim().to_owned();
        if query.is_empty() {
            self.search_hits = None;
            return;
        }
        match self.collect_matches(&query) {
            Ok(hits) => {
                if let Some(first) = hits.first() {
                    self.goto_page(*first);
                }
                self.search_hits = Some(hits);
            }
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    /// All pages containing the query, in document order starting from the
    /// current page (wrapping once).
    fn collect_matches(&self, query: &str) -> Result<Vec<usize>, PdfError> {
        let Some(document) = self.document.as_ref() else {
            return Ok(Vec::new());
        };
        let mut session = SearchSession::new(document.as_ref(), query, self.session.page);
        let mut seen = Vec::new();
        while let Some(hit) = session.next()? {
            if seen.contains(&hit) {
                break;
            }
            seen.push(hit);
        }
        let mut pages = Vec::new();
        for (page, _) in seen {
            if pages.last() != Some(&page) {
                pages.push(page);
            }
        }
        Ok(pages)
    }

    // --- rendering -------------------------------------------------------

    fn collect_results(&mut self, ctx: &egui::Context) {
        let results = match &self.pipeline {
            Some(pipeline) => pipeline.poll(),
            None => return,
        };
        for result in results {
            match result {
                RenderResult::Ready { request, image } => {
                    self.pending.remove(&request.key());
                    if request.scale_q == self.current_scale_q(ctx) {
                        self.promote_texture(ctx, request.page, request.scale_q, Some(&image));
                    }
                    let PageImage {
                        width,
                        height,
                        rgba,
                    } = image;
                    self.cache.insert(
                        CacheKey {
                            page: request.page,
                            scale_q: request.scale_q,
                        },
                        width,
                        height,
                        rgba,
                    );
                    ctx.request_repaint();
                }
                RenderResult::Failed { request, error } => {
                    self.pending.remove(&request.key());
                    self.failed.insert(request.key());
                    self.error = Some(format!("rendering page {}: {error}", request.page + 1));
                }
            }
        }
    }

    fn current_scale(&self, pixels_per_point: f32) -> f32 {
        f32::from(self.zoom_pct) / 100.0 * pixels_per_point
    }

    fn current_scale_q(&self, ctx: &egui::Context) -> u16 {
        (self.current_scale(ctx.pixels_per_point()) * 100.0).round() as u16
    }

    /// Rebuild the layout when the available width or zoom preference changed.
    /// Page sizes are immutable per document, so they are fetched once at open
    /// and reused across relayouts.
    fn ensure_layout(&mut self, avail_w: f32) -> bool {
        let key = (avail_w, self.session.zoom);
        if self.layout_key == Some(key) {
            return true;
        }
        if self.page_sizes.is_empty() {
            return false;
        }
        self.layout = Layout::build(&self.page_sizes, self.session.zoom, avail_w);
        self.zoom_pct = layout::quantize_nearest(self.layout.zoom * 100.0);
        self.layout_key = Some(key);
        true
    }

    /// Fetch every page's point size once; a backend failure here is fatal to
    /// layout and surfaces as an error banner.
    fn load_page_sizes(&mut self) {
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let count = document.page_count();
        let mut sizes = Vec::with_capacity(count);
        for page in 0..count {
            match document.page_size(page) {
                Ok(size) => sizes.push(size),
                Err(err) => {
                    self.error = Some(format!("page {}: {err}", page + 1));
                    self.page_sizes.clear();
                    return;
                }
            }
        }
        self.page_sizes = sizes;
    }

    /// Render the current page synchronously so opening never shows a blank
    /// canvas; later pages stream in from the worker.
    fn prime_current_page(&mut self, ctx: &egui::Context) {
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let count = document.page_count();
        if count == 0 {
            return;
        }
        let page = self.session.page.min(count - 1);
        let scale_q = self.current_scale_q(ctx);
        if self
            .textures
            .get(&page)
            .is_some_and(|t| t.scale_q == scale_q)
        {
            return;
        }
        let key = CacheKey { page, scale_q };
        if self.cache.contains(key) {
            self.promote_texture(ctx, page, scale_q, None);
            return;
        }
        match document.render_page(page, self.current_scale(ctx.pixels_per_point())) {
            Ok(image) => {
                self.promote_texture(ctx, page, scale_q, Some(&image));
                let PageImage {
                    width,
                    height,
                    rgba,
                } = image;
                self.cache.insert(key, width, height, rgba);
            }
            Err(err) => {
                self.failed.insert(key);
                self.error = Some(format!("rendering page {}: {err}", page + 1));
            }
        }
    }

    /// Recolor-at-promotion: clone a cached original, map it onto the theme's
    /// page colors, then load once per slot or reuse via `TextureHandle::set`.
    fn promote_texture(
        &mut self,
        ctx: &egui::Context,
        page: usize,
        scale_q: u16,
        fallback: Option<&PageImage>,
    ) {
        let pixels = match self.cache.get(CacheKey { page, scale_q }) {
            Some(img) => (img.width, img.height, img.rgba.to_vec()),
            None => match fallback {
                Some(image) => (image.width, image.height, image.rgba.clone()),
                None => return,
            },
        };
        let (width, height, mut rgba) = pixels;
        recolor(&mut rgba, self.theme.page_bg, self.theme.page_fg);
        let image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        let options = egui::TextureOptions::LINEAR;
        match self.textures.get_mut(&page) {
            Some(slot) if slot.scale_q == scale_q => slot.handle.set(image, options),
            _ => {
                let handle = ctx.load_texture(format!("page-{page}"), image, options);
                self.textures.insert(page, PageTexture { scale_q, handle });
            }
        }
    }

    /// Queue renders for the visible pages plus a ±2 prefetch, highest
    /// priority first (current > adjacent > prefetch > rest of viewport).
    /// Cached pages promote directly without rendering.
    fn request_pages(&mut self, ctx: &egui::Context, visible: Range<usize>) {
        let count = self.page_count();
        if count == 0 {
            return;
        }
        let current = self.session.page.min(count - 1);
        let mut candidates = vec![current];
        for delta in [-1isize, 1, -2, 2] {
            let page = current as isize + delta;
            if (0..count as isize).contains(&page) {
                candidates.push(page as usize);
            }
        }
        for page in visible {
            if !candidates.contains(&page) {
                candidates.push(page);
            }
        }

        let scale_q = self.current_scale_q(ctx);
        if self.failed_scale_q != scale_q {
            self.failed.clear();
            self.failed_scale_q = scale_q;
        }
        let scale = self.current_scale(ctx.pixels_per_point());
        let mut wanted = Vec::new();
        for page in candidates {
            let key = CacheKey { page, scale_q };
            if self.pending.contains(&key)
                || self.failed.contains(&key)
                || self
                    .textures
                    .get(&page)
                    .is_some_and(|t| t.scale_q == scale_q)
            {
                continue;
            }
            if self.cache.contains(key) {
                self.promote_texture(ctx, page, scale_q, None);
                continue;
            }
            wanted.push(RenderRequest {
                page,
                scale_q,
                scale,
            });
            self.pending.insert(key);
        }
        if !wanted.is_empty()
            && let Some(pipeline) = &self.pipeline
        {
            pipeline.submit(&wanted);
        }
    }

    fn prune_textures(&mut self) {
        let current = self.session.page;
        self.textures.retain(|&page, _| {
            page.saturating_sub(current) <= TEXTURE_KEEP_AROUND
                && current.saturating_sub(page) <= TEXTURE_KEEP_AROUND
        });
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        if !self.ensure_layout(ui.available_width()) {
            return;
        }
        let ctx = ui.ctx().clone();
        if !self.primed {
            self.primed = true;
            self.prime_current_page(&ctx);
        }
        // Apply a saved position exactly once heights are known: anchor at
        // least to the saved page, refined by the whole-document fraction
        // (design guide §7) when it points further down.
        if let Some(frac) = self.restore_frac.take() {
            let page_top = self
                .layout
                .rects
                .get(self.session.page)
                .map_or(0.0, |rect| (rect.y - GAP).max(0.0));
            let target = if frac > 0.0 {
                let span = (self.layout.total_height - ui.clip_rect().height()).max(0.0);
                ((frac * f64::from(span)) as f32).max(page_top)
            } else {
                page_top
            };
            self.pending_scroll = Some(target);
        }
        let jump = self.pending_scroll.take();

        let mut area = egui::ScrollArea::vertical()
            .id_salt("page_canvas")
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded);
        if let Some(y) = jump {
            area = area.vertical_scroll_offset(y);
        }

        struct View {
            visible: Range<usize>,
            center_y: f32,
            clip_h: f32,
        }
        let mut view = View {
            visible: 0..0,
            center_y: 0.0,
            clip_h: 0.0,
        };

        let output = area.show(ui, |ui| {
            let content_w = ui.available_width();
            let (content_rect, _) = ui.allocate_exact_size(
                egui::vec2(content_w, self.layout.total_height),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(content_rect);
            let clip = ui.clip_rect();
            let top = clip.top() - content_rect.top();
            view.visible = self.layout.visible_range(top, clip.height());
            view.center_y = top + clip.height() * 0.5;
            view.clip_h = clip.height();

            let panel = color_of(self.theme.panel_bg);
            let fg = color_of(self.theme.ui_fg);
            let border = stroke(1.0, fg.gamma_multiply(0.25));
            let hint_font = egui::FontId::proportional(14.0);
            for page in view.visible.clone() {
                let rect = self.layout.rects[page];
                let screen = egui::Rect::from_min_size(
                    content_rect.min + egui::vec2(rect.x, rect.y),
                    egui::vec2(rect.w, rect.h),
                );
                match self.textures.get(&page) {
                    Some(texture) => {
                        painter.image(
                            texture.handle.id(),
                            screen,
                            uv_unit_rect(),
                            egui::Color32::WHITE,
                        );
                    }
                    None => {
                        painter.rect_filled(screen, 4.0, panel);
                        painter.text(
                            screen.center(),
                            egui::Align2::CENTER_CENTER,
                            "rendering…",
                            hint_font.clone(),
                            fg.gamma_multiply(0.6),
                        );
                    }
                }
                painter.rect_stroke(screen, 2.0, border);
            }
        });

        if self.page_count() > 0 {
            if let Some(page) = self.layout.page_at(view.center_y) {
                self.session.page = page;
            }
            self.request_pages(&ctx, view.visible);
            self.prune_textures();
            let span = (self.layout.total_height - view.clip_h).max(1.0);
            self.session.scroll_frac =
                (f64::from(output.state.offset.y) / f64::from(span)).clamp(0.0, 1.0);
        }
    }

    // --- chrome ----------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("☰", |ui| {
                if ui.button("Open File…   Ctrl+O").clicked() {
                    ui.close_menu();
                    self.open_dialog();
                }
                if ui.button("Save State   Ctrl+S").clicked() {
                    ui.close_menu();
                    self.save_state();
                }
                self.theme_menu(ui);
                if ui.button("About Candi").clicked() {
                    ui.close_menu();
                    self.about_open = true;
                }
            });

            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    if !self.filename.is_empty() {
                        ui.strong(&self.filename);
                    }
                },
            );

            let count = self.page_count();
            let current = self.session.page;
            if ui
                .add_enabled(count > 0 && current > 0, egui::Button::new("‹"))
                .clicked()
            {
                self.goto_page(current - 1);
            }
            if count > 0 {
                ui.label(format!("{} / {}", current + 1, count));
            } else {
                ui.label("–");
            }
            if ui
                .add_enabled(count > 0 && current + 1 < count, egui::Button::new("›"))
                .clicked()
            {
                self.goto_page(current + 1);
            }

            if ui.selectable_label(self.search_open, "Search").clicked() {
                self.search_open = !self.search_open;
                self.focus_search = self.search_open;
            }
            if self.search_open {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .desired_width(SEARCH_FIELD_WIDTH)
                        .hint_text("Find in document"),
                );
                if self.focus_search {
                    field.request_focus();
                    self.focus_search = false;
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    self.run_search();
                }
                match &self.search_hits {
                    Some(hits) => ui.label(format!("{} matches", hits.len())),
                    None => ui.label("(Enter)"),
                };
            }
        });
    }

    fn theme_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Theme", |ui| {
            for name in BUILTIN_NAMES {
                let label = if self.theme.name == *name {
                    format!("✓ {name}")
                } else {
                    format!("   {name}")
                };
                if ui.button(label).clicked() {
                    ui.close_menu();
                    self.set_theme(name);
                }
            }
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("DOCUMENT").weak().small());
        ui.label(egui::RichText::new(&self.filename).strong());
        ui.separator();

        let rows = [
            (SidebarSection::Contents, "Contents", None),
            (
                SidebarSection::Bookmarks,
                "Bookmarks",
                Some(self.session.bookmarks.len()),
            ),
            (
                SidebarSection::Search,
                "Search",
                self.search_hits.as_deref().map(<[usize]>::len),
            ),
        ];
        for (section, label, count) in rows {
            let text = match count {
                Some(n) => format!("{label} ({n})"),
                None => label.to_owned(),
            };
            if ui.selectable_label(self.section == section, text).clicked() {
                self.section = section;
            }
        }
        ui.separator();

        let note = match self.section {
            SidebarSection::Contents => "Contents arrive in a later slice.",
            SidebarSection::Bookmarks => "Bookmark management arrives in a later slice.",
            SidebarSection::Search => "Use the search field in the top bar (Ctrl+F).",
        };
        ui.label(egui::RichText::new(note).weak());
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Theme")
                .selected_text(&self.theme.name)
                .show_ui(ui, |ui| {
                    for name in BUILTIN_NAMES {
                        if ui
                            .selectable_label(self.theme.name == *name, name)
                            .clicked()
                        {
                            self.set_theme(name);
                        }
                    }
                });
            ui.add_enabled(false, egui::Button::new("Edit…"));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Fit width").clicked() {
                    self.session.zoom = ZoomMode::FitWidth;
                }
                if ui.button("+").clicked() {
                    self.zoom_step(5);
                }
                if ui.button(format!("{}%", self.zoom_pct)).clicked() {
                    self.session.zoom = ZoomMode::FitWidth;
                }
                if ui.button("−").clicked() {
                    self.zoom_step(-5);
                }
            });
        });
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("About Candi")
            .open(&mut self.about_open)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Candi").strong().size(18.0));
                ui.label("A minimal PDF reader.");
                ui.label("v0.1 — AGPL-3.0");
            });
    }
}

fn read_shortcuts(ctx: &egui::Context) -> Shortcuts {
    let plain = !ctx.wants_keyboard_input();
    ctx.input(|input| {
        let ctrl = input.modifiers.ctrl;
        Shortcuts {
            open: ctrl && input.key_pressed(Key::O),
            save: ctrl && input.key_pressed(Key::S),
            sidebar: ctrl && input.key_pressed(Key::B),
            find: ctrl && input.key_pressed(Key::F),
            zoom_in: plain && (input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals)),
            zoom_out: plain && input.key_pressed(Key::Minus),
            fit_width: plain && input.key_pressed(Key::Num0),
            cycle_theme: plain && input.key_pressed(Key::T),
            quit: plain && input.key_pressed(Key::Q),
            escape: input.key_pressed(Key::Escape),
            prev_page: plain
                && (input.key_pressed(Key::ArrowLeft) || input.key_pressed(Key::PageUp)),
            next_page: plain
                && (input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::PageDown)),
        }
    })
}

impl ReaderApp {
    fn apply_shortcuts(&mut self, ctx: &egui::Context, s: Shortcuts) {
        if s.open {
            self.open_dialog();
        }
        if s.save {
            self.save_state();
        }
        if s.sidebar {
            self.sidebar_open = !self.sidebar_open;
        }
        if s.find {
            self.search_open = true;
            self.focus_search = true;
        }
        if s.zoom_in {
            self.zoom_step(5);
        }
        if s.zoom_out {
            self.zoom_step(-5);
        }
        if s.fit_width {
            self.session.zoom = ZoomMode::FitWidth;
        }
        if s.cycle_theme {
            let next = BUILTIN_NAMES
                .iter()
                .position(|name| *name == self.theme.name)
                .map_or(0, |idx| (idx + 1) % BUILTIN_NAMES.len());
            self.set_theme(BUILTIN_NAMES[next]);
        }
        if s.quit {
            self.save_state();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if s.escape {
            if self.about_open {
                self.about_open = false;
            } else if self.search_open {
                self.search_open = false;
                self.search_hits = None;
                self.search_query.clear();
            }
        }
        if s.prev_page {
            self.goto_page(self.session.page.saturating_sub(1));
        }
        if s.next_page {
            self.goto_page(self.session.page + 1);
        }
    }
}

fn color_of(color: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a())
}

fn stroke(width: f32, color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(width, color)
}

fn uv_unit_rect() -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
}

fn filename_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document")
        .to_owned()
}

/// Map theme tokens onto egui visuals; called whenever the theme changes.
fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let fg = color_of(theme.ui_fg);
    let accent = color_of(theme.accent);
    let selection = color_of(theme.selection);
    let dim_border = stroke(1.0, fg.gamma_multiply(0.25));
    let accent_stroke = stroke(1.0, accent);

    ctx.style_mut(|style| {
        let visuals = &mut style.visuals;
        visuals.panel_fill = color_of(theme.panel_bg);
        visuals.window_fill = color_of(theme.ui_bg);
        visuals.extreme_bg_color = color_of(theme.page_bg);
        visuals.faint_bg_color = color_of(theme.ui_bg);
        visuals.override_text_color = Some(fg);
        visuals.hyperlink_color = accent;
        visuals.selection.bg_fill = selection;
        visuals.selection.stroke = accent_stroke;

        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = color_of(theme.ui_bg);
        widgets.noninteractive.fg_stroke = stroke(1.0, fg);
        widgets.noninteractive.bg_stroke = dim_border;
        widgets.inactive.fg_stroke = stroke(1.0, fg);
        widgets.inactive.weak_bg_fill = color_of(theme.panel_bg);
        widgets.inactive.bg_stroke = dim_border;
        widgets.hovered.fg_stroke = stroke(1.0, fg);
        widgets.hovered.weak_bg_fill = selection;
        widgets.hovered.bg_stroke = accent_stroke;
        widgets.active.fg_stroke = stroke(1.0, fg);
        widgets.active.weak_bg_fill = selection;
        widgets.active.bg_fill = selection;
    });
}

impl eframe::App for ReaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.applied_theme != self.theme.name {
            apply_theme(ctx, &self.theme);
            self.applied_theme.clone_from(&self.theme.name);
        }

        let shortcuts = read_shortcuts(ctx);
        self.collect_results(ctx);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| self.top_bar(ui));
        if self.sidebar_open {
            egui::SidePanel::left("sidebar")
                .default_width(SIDEBAR_WIDTH)
                .resizable(false)
                .show(ctx, |ui| self.sidebar(ui));
        }
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| self.bottom_bar(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = self.error.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(0xE5, 0x48, 0x4D), error);
                    if ui.small_button("dismiss").clicked() {
                        self.error = None;
                    }
                });
                ui.separator();
            }
            if self.document.is_some() {
                self.show_canvas(ui);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open a PDF to start reading (Ctrl+O).");
                });
            }
        });

        self.about_window(ctx);
        self.apply_shortcuts(ctx, shortcuts);

        // Completed renders must wake the UI even when no input arrives;
        // poll at a fixed cadence while anything is outstanding.
        if !self.pending.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(path) = self.path.as_deref()
            && let Err(err) = save_session(path, &self.session)
        {
            eprintln!("candi: saving state on exit: {err}");
        }
    }
}
