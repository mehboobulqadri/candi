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
use candi_core::{SearchSession, SessionState, ZoomMode, normalize_reader_text};
use candi_pdf::{BackendKind, Document, Error as PdfError, PageImage};
use candi_theme::{BUILTIN_NAMES, Color, Theme, builtin, parse, recolor, to_yaml};
use eframe::egui;
use egui::Key;

use crate::highlight::yaml_job;
use crate::render::cache::{CacheKey, DEFAULT_BUDGET_BYTES, ImageCache};
use crate::render::layout::{self, GAP, Layout};
use crate::render::pipeline::{Pipeline, RenderRequest, RenderResult};
use crate::sidebar::{SearchHit, SidebarSection, TocRow, date_only, extract_snippet, flatten_toc};

/// Pages kept as textures around the current page; texture memory outside this
/// window is released while the original bitmaps stay cached.
const TEXTURE_KEEP_AROUND: usize = 6;
const SIDEBAR_WIDTH: f32 = 260.0;
/// Corner rounding shared by page shadow, placeholder fill, and border.
const PAGE_ROUNDING: f32 = 3.0;
/// Sidebar contents indentation per outline nesting level.
const INDENT_PER_LEVEL: f32 = 12.0;
const ERROR_RED: egui::Color32 = egui::Color32::from_rgb(0xE5, 0x48, 0x4D);

/// A promoted GPU texture for one page plus the scale it was rendered at.
struct PageTexture {
    scale_q: u16,
    handle: egui::TextureHandle,
}

/// Center-pane theme editor state: the YAML buffer plus its last parse
/// outcome. Openness is encoded by [`ReaderApp`] holding
/// `Option<ThemeEditor>`; applying a parsed theme stays with the caller so
/// this stays egui-free and unit-testable.
struct ThemeEditor {
    buffer: String,
    error: Option<String>,
}

impl ThemeEditor {
    fn open(theme: &Theme) -> Self {
        Self {
            buffer: to_yaml(theme),
            error: None,
        }
    }

    /// Swap in the edited buffer and reparse immediately; `Some` carries the
    /// theme to apply, `None` leaves the caller's last-good theme in place.
    fn edit(&mut self, buffer: String) -> Option<Theme> {
        self.buffer = buffer;
        match parse(&self.buffer) {
            Ok(theme) => {
                self.error = None;
                Some(theme)
            }
            Err(err) => {
                self.error = Some(err.to_string());
                None
            }
        }
    }
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
    focus_search: bool,
    search_query: String,
    search_hits: Option<Vec<SearchHit>>,
    /// Flattened table of contents, loaded once per open; empty = none.
    toc_rows: Vec<TocRow>,
    about_open: bool,
    /// Live theme editor; `Some` while the center pane shows it.
    editor: Option<ThemeEditor>,

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
            cache: ImageCache::new(DEFAULT_BUDGET_BYTES),
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
            focus_search: false,
            search_query: String::new(),
            search_hits: None,
            toc_rows: Vec::new(),
            about_open: false,
            editor: None,
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
        self.toc_rows.clear();
        match open_session(&path, self.backend) {
            Ok(opened) => {
                self.cache = ImageCache::new(DEFAULT_BUDGET_BYTES);
                self.pending.clear();
                self.failed.clear();
                self.failed_scale_q = 0;
                self.textures.clear();
                self.pipeline = None;
                self.layout_key = None;
                self.page_sizes.clear();
                self.pending_scroll = None;
                self.primed = false;
                self.sidebar_open = true;

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
                self.load_outline();
            }
            Err(err) => {
                self.error = Some(err.to_string());
            }
        }
    }

    /// Switch the active built-in theme. Visuals are re-applied at the top of
    /// the next [`ReaderApp::update`]; texture slots are dropped so pages
    /// re-promote from their cached originals in the new colors. Closes the
    /// theme editor: a dropdown/menu switch means the buffer is no longer
    /// authoritative.
    fn set_theme(&mut self, name: &str) {
        self.theme =
            builtin(name).unwrap_or_else(|| builtin("Light").expect("built-in Light parses"));
        self.session.theme = self.theme.name.clone();
        self.textures.clear();
        self.editor = None;
    }

    /// Open the center-pane theme editor seeded with the active theme;
    /// re-opening while open keeps the user's buffer.
    fn open_theme_editor(&mut self) {
        if self.editor.is_none() {
            self.editor = Some(ThemeEditor::open(&self.theme));
        }
    }

    /// Swap in a theme parsed from the editor buffer. Same mechanism as
    /// [`ReaderApp::set_theme`], but `applied_theme` is cleared so visuals are
    /// restyled even when the edited name is unchanged.
    fn apply_edited_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.session.theme = self.theme.name.clone();
        self.textures.clear();
        self.applied_theme.clear();
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
        if let Some(path) = crate::pdf_dialog().pick_file() {
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
                    self.goto_page(first.page);
                }
                self.search_hits = Some(hits);
            }
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    /// Every match in the document as a result row, in document order.
    /// `SearchSession` cycles its cursor once the scan is complete, so the
    /// collection stops when the first hit comes around again.
    fn collect_matches(&self, query: &str) -> Result<Vec<SearchHit>, PdfError> {
        let Some(document) = self.document.as_ref() else {
            return Ok(Vec::new());
        };
        let mut session = SearchSession::new(document.as_ref(), query, 0);
        let mut first: Option<(usize, usize)> = None;
        while let Some(hit) = session.next()? {
            if first.is_none() {
                first = Some(hit);
            } else if first == Some(hit) {
                break;
            }
        }

        // Hits arrive grouped by page in ascending order; the page text is
        // fetched once per group. Offsets index into exactly this
        // normalized+lowercased form of the page text.
        let needle_len = query.to_lowercase().len();
        let mut hits = Vec::new();
        for group in session.results().chunk_by(|a, b| a.0 == b.0) {
            let page = group[0].0;
            let text = normalize_reader_text(&document.page_text(page)?).to_lowercase();
            for &(_, offset) in group {
                hits.push(SearchHit {
                    page,
                    snippet: extract_snippet(&text, offset, needle_len),
                });
            }
        }
        Ok(hits)
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

    /// Fetch the outline once per open. An empty result means the document
    /// has no usable table of contents; a backend failure surfaces as an
    /// error banner with the sidebar falling back to its empty state.
    fn load_outline(&mut self) {
        self.toc_rows.clear();
        let Some(document) = self.document.as_ref() else {
            return;
        };
        match document.outline() {
            Ok(items) => self.toc_rows = flatten_toc(&items),
            Err(err) => self.error = Some(format!("table of contents: {err}")),
        }
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
            && !pipeline.submit(&wanted)
        {
            self.renderer_stopped();
        }
    }

    /// The worker thread died; nothing will ever drain `pending`. Drop the
    /// pipeline so placeholders stop promising progress and surface the
    /// failure instead of spinning silently.
    fn renderer_stopped(&mut self) {
        self.pipeline = None;
        self.pending.clear();
        self.error = Some("renderer stopped; restart Candi to keep reading".into());
    }

    fn prune_textures(&mut self) {
        let current = self.session.page;
        self.textures.retain(|&page, _| {
            page.saturating_sub(current) <= TEXTURE_KEEP_AROUND
                && current.saturating_sub(page) <= TEXTURE_KEEP_AROUND
        });
    }

    /// Center-pane theme editor: header row, parse-error banner when the
    /// buffer is bad, and a monospace TextEdit filling the rest. Every edit
    /// reparses immediately; a good buffer swaps the live theme.
    fn show_theme_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong(format!("Theme — {}", self.theme.name));
            ui.label(egui::RichText::new("Edits apply live · Esc to close").weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.editor = None;
                }
            });
        });
        ui.label(
            egui::RichText::new(
                "Only built-in names persist; unknown names load as Light next session.",
            )
            .weak()
            .small(),
        );

        let edited = {
            let Some(editor) = self.editor.as_mut() else {
                return;
            };
            if let Some(err) = &editor.error {
                ui.colored_label(ERROR_RED, err);
            }
            let mut buffer = std::mem::take(&mut editor.buffer);
            let mut layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                let job = yaml_job(text, &self.theme);
                ui.fonts(|f| f.layout_job(job))
            };
            let edit_box = egui::TextEdit::multiline(&mut buffer)
                .font(egui::TextStyle::Monospace)
                .layouter(&mut layouter);
            let response = ui.add_sized([ui.available_width(), ui.available_height()], edit_box);
            if response.changed() {
                editor.edit(buffer)
            } else {
                editor.buffer = buffer;
                None
            }
        };
        if let Some(theme) = edited {
            self.apply_edited_theme(theme);
        }
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
            let paper = color_of(self.theme.page_bg);
            let fg = color_of(self.theme.ui_fg);
            let border = egui::Stroke::new(1.0_f32, fg.gamma_multiply(0.25));
            // Soft drop shadow so pages read as sheets of paper on the
            // panel-colored backdrop, even when page and backdrop are close
            // (Light).
            let mut shadow = egui::epaint::RectShape::filled(
                egui::Rect::ZERO,
                egui::Rounding::same(PAGE_ROUNDING),
                egui::Color32::from_black_alpha(56),
            );
            shadow.blur_width = 9.0;
            let hint_font = egui::FontId::proportional(14.0);
            painter.rect_filled(content_rect, 0.0, panel);
            for page in view.visible.clone() {
                let rect = self.layout.rects[page];
                let screen = egui::Rect::from_min_size(
                    content_rect.min + egui::vec2(rect.x, rect.y),
                    egui::vec2(rect.w, rect.h),
                );
                painter.add(egui::epaint::RectShape {
                    rect: screen.expand2(egui::vec2(3.0, 5.0)),
                    ..shadow
                });
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
                        painter.rect_filled(screen, PAGE_ROUNDING, paper);
                        painter.text(
                            screen.center(),
                            egui::Align2::CENTER_CENTER,
                            "rendering…",
                            hint_font.clone(),
                            fg.gamma_multiply(0.6),
                        );
                    }
                }
                painter.rect_stroke(screen, PAGE_ROUNDING, border);
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
        // Roomier buttons: the hamburger and page-nav arrows were cramped at
        // the default padding.
        ui.style_mut().spacing.button_padding = egui::vec2(10.0, 4.0);
        let row_w = ui.available_width();
        ui.columns(3, |columns| {
            columns[0].horizontal(|ui| {
                ui.menu_button(egui::RichText::new("☰").size(16.0), |ui| {
                    if ui.button("Open File…   Ctrl+O").clicked() {
                        ui.close_menu();
                        self.open_dialog();
                    }
                    if ui.button("Save State   Ctrl+S").clicked() {
                        ui.close_menu();
                        self.save_state();
                    }
                    self.theme_menu(ui);
                    if ui.button("Edit theme YAML…   Ctrl+E").clicked() {
                        ui.close_menu();
                        self.open_theme_editor();
                    }
                    if ui.button("About Candi").clicked() {
                        ui.close_menu();
                        self.about_open = true;
                    }
                });

                // Title sits next to the burger and truncates with an
                // ellipsis instead of consuming the row.
                if !self.filename.is_empty() {
                    let title_max = ui.available_width().min(row_w * 0.25);
                    let title =
                        egui::Label::new(egui::RichText::new(&self.filename).strong()).truncate();
                    ui.add_sized([title_max, 20.0], title);
                }
            });

            columns[1].horizontal(|ui| {
                let count = self.page_count();
                let current = self.session.page;
                let counter = if count > 0 {
                    format!("{} / {}", current + 1, count)
                } else {
                    "–".to_owned()
                };
                // Center the cluster by its measured width; a justified
                // layout would stretch the buttons into bars.
                let font = egui::FontId::proportional(14.0);
                let advance = |text: &str| {
                    ui.painter()
                        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                };
                let pad = ui.spacing().button_padding.x * 2.0;
                let gap = ui.spacing().item_spacing.x;
                let cluster_w =
                    pad + advance("‹") + pad + advance("›") + advance(&counter) + gap * 2.0;
                ui.add_space(((ui.available_width() - cluster_w) * 0.5).max(0.0));

                if ui
                    .add_enabled(count > 0 && current > 0, egui::Button::new("‹"))
                    .clicked()
                {
                    self.goto_page(current - 1);
                }
                ui.label(egui::RichText::new(counter).font(font));
                if ui
                    .add_enabled(count > 0 && current + 1 < count, egui::Button::new("›"))
                    .clicked()
                {
                    self.goto_page(current + 1);
                }
            });

            columns[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sidebar_label = if self.sidebar_open { "◧" } else { "▤" };
                if ui
                    .button(sidebar_label)
                    .on_hover_text("Toggle sidebar (Ctrl+B)")
                    .clicked()
                {
                    self.sidebar_open = !self.sidebar_open;
                }
            });
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

        let contents_count = (!self.toc_rows.is_empty()).then_some(self.toc_rows.len());
        let bookmark_count =
            (!self.session.bookmarks.is_empty()).then_some(self.session.bookmarks.len());
        let search_count = self.search_hits.as_deref().map(<[SearchHit]>::len);
        let rows = [
            (SidebarSection::Contents, "Contents", contents_count),
            (SidebarSection::Bookmarks, "Bookmarks", bookmark_count),
            (SidebarSection::Search, "Search", search_count),
        ];
        for (section, label, count) in rows {
            let active = self.section == section;
            let text = match count {
                Some(n) => format!("{label} ({n})"),
                None => label.to_owned(),
            };
            let mut rich = egui::RichText::new(text);
            if active {
                rich = rich.color(color_of(self.theme.accent)).strong();
            }
            if ui.selectable_label(active, rich).clicked() {
                self.section = section;
                if section == SidebarSection::Search {
                    self.focus_search = true;
                }
            }
        }
        ui.separator();

        let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        match self.section {
            SidebarSection::Contents => {
                area.id_salt("sidebar_contents")
                    .show(ui, |ui| self.show_contents(ui));
            }
            SidebarSection::Bookmarks => {
                area.id_salt("sidebar_bookmarks")
                    .show(ui, |ui| self.show_bookmarks(ui));
            }
            SidebarSection::Search => {
                area.id_salt("sidebar_search")
                    .show(ui, |ui| self.show_search(ui));
            }
        };
    }

    fn show_contents(&mut self, ui: &mut egui::Ui) {
        if self.toc_rows.is_empty() {
            ui.label(egui::RichText::new("No table of contents").weak());
            return;
        }
        let mut jump = None;
        for row in &self.toc_rows {
            if toc_row_ui(ui, row).clicked() {
                jump = Some(row.page);
            }
        }
        if let Some(page) = jump {
            self.goto_page(page);
        }
    }

    fn show_bookmarks(&mut self, ui: &mut egui::Ui) {
        if self.page_count() > 0 && ui.button("Add bookmark").clicked() {
            self.session.add_bookmark(self.session.page);
        }
        if self.session.bookmarks.is_empty() {
            let hint = if self.page_count() > 0 {
                "No bookmarks — press B to mark this page"
            } else {
                "No bookmarks"
            };
            ui.label(egui::RichText::new(hint).weak());
            return;
        }
        let mut jump = None;
        let mut remove = None;
        for bookmark in &self.session.bookmarks {
            ui.horizontal(|ui| {
                if click_row(
                    ui,
                    format!(
                        "p. {} · {}",
                        bookmark.page + 1,
                        date_only(&bookmark.created_at)
                    ),
                )
                .clicked()
                {
                    jump = Some(bookmark.page);
                }
                if ui.small_button("✕").clicked() {
                    remove = Some(bookmark.page);
                }
            });
        }
        if let Some(page) = jump {
            self.goto_page(page);
        }
        if let Some(page) = remove {
            self.session.remove_bookmark(page);
        }
    }

    fn show_search(&mut self, ui: &mut egui::Ui) {
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Find in document")
                .desired_width(f32::INFINITY),
        );
        if self.focus_search {
            field.request_focus();
            self.focus_search = false;
        }
        if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            self.run_search();
        }

        let mut jump = None;
        match self.search_hits.as_deref() {
            None => {
                ui.label(egui::RichText::new("Type a query and press Enter").weak());
            }
            Some([]) => {
                ui.label(egui::RichText::new("No matches").weak());
            }
            Some(hits) => {
                for hit in hits {
                    if click_row(ui, format!("p. {} — {}", hit.page + 1, hit.snippet)).clicked() {
                        jump = Some(hit.page);
                    }
                }
            }
        };
        if let Some(page) = jump {
            self.goto_page(page);
        }
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Label first, then the value it names.
            ui.label("Theme");
            egui::ComboBox::from_id_salt("theme_combo")
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
            if ui.button("Edit…").clicked() {
                self.open_theme_editor();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_zoom = self.page_count() > 0;
                if ui.add_enabled(can_zoom, egui::Button::new("+")).clicked() {
                    self.zoom_step(5);
                }
                // "Fit" while fit-width is active; clicking returns to it from
                // a percent zoom. − / + always step from the effective zoom
                // and land in percent mode.
                let zoom_label = match self.session.zoom {
                    ZoomMode::FitWidth => "Fit".to_owned(),
                    ZoomMode::Percent(_) => format!("{}%", self.zoom_pct),
                };
                if ui
                    .add_enabled(can_zoom, egui::Button::new(zoom_label))
                    .clicked()
                {
                    self.session.zoom = ZoomMode::FitWidth;
                }
                if ui.add_enabled(can_zoom, egui::Button::new("−")).clicked() {
                    self.zoom_step(-5);
                }
                let marked = self
                    .session
                    .bookmarks
                    .iter()
                    .any(|bookmark| bookmark.page == self.session.page);
                if ui
                    .add_enabled(
                        self.page_count() > 0,
                        egui::Button::new(if marked { "★" } else { "☆" }),
                    )
                    .on_hover_text("Bookmark this page (B)")
                    .clicked()
                {
                    self.session.toggle_bookmark(self.session.page);
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
                ui.label(format!("v{} — AGPL-3.0", env!("CARGO_PKG_VERSION")));
            });
    }
}

impl ReaderApp {
    /// Handle the frame's keyboard input. Plain keys are ignored while any
    /// widget wants keyboard input (text fields, menus).
    fn handle_input(&mut self, ctx: &egui::Context) {
        let plain = !ctx.wants_keyboard_input();
        ctx.input(|input| {
            let ctrl = input.modifiers.ctrl;
            if ctrl && input.key_pressed(Key::O) {
                self.open_dialog();
            }
            if ctrl && input.key_pressed(Key::S) {
                self.save_state();
            }
            if ctrl && input.key_pressed(Key::B) {
                self.sidebar_open = !self.sidebar_open;
            }
            if ctrl && input.key_pressed(Key::F) {
                self.sidebar_open = true;
                self.section = SidebarSection::Search;
                self.focus_search = true;
            }
            if ctrl && input.key_pressed(Key::E) {
                if self.editor.is_some() {
                    self.editor = None;
                } else {
                    self.open_theme_editor();
                }
            }
            if plain
                && (input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals))
                && self.page_count() > 0
            {
                self.zoom_step(5);
            }
            if plain && input.key_pressed(Key::Minus) && self.page_count() > 0 {
                self.zoom_step(-5);
            }
            if plain && input.key_pressed(Key::Num0) && self.page_count() > 0 {
                self.session.zoom = ZoomMode::FitWidth;
            }
            if plain && input.key_pressed(Key::T) {
                let next = BUILTIN_NAMES
                    .iter()
                    .position(|name| *name == self.theme.name)
                    .map_or(0, |idx| (idx + 1) % BUILTIN_NAMES.len());
                self.set_theme(BUILTIN_NAMES[next]);
            }
            if plain && input.key_pressed(Key::B) && self.page_count() > 0 {
                self.session.toggle_bookmark(self.session.page);
            }
            if plain && input.key_pressed(Key::Q) {
                self.save_state();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if input.key_pressed(Key::Escape) {
                if self.about_open {
                    self.about_open = false;
                } else if self.editor.is_some() {
                    self.editor = None;
                } else if self.sidebar_open
                    && self.section == SidebarSection::Search
                    && !self.search_query.is_empty()
                {
                    self.search_hits = None;
                    self.search_query.clear();
                }
            }
            if plain && (input.key_pressed(Key::ArrowLeft) || input.key_pressed(Key::PageUp)) {
                self.goto_page(self.session.page.saturating_sub(1));
            }
            if plain && (input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::PageDown)) {
                self.goto_page(self.session.page + 1);
            }
        });
    }
}

fn color_of(color: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a())
}

/// One contents row: full-width clickable title indented 12 pt per outline
/// level, truncated when too long.
fn toc_row_ui(ui: &mut egui::Ui, row: &TocRow) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_space(INDENT_PER_LEVEL * row.depth as f32);
        click_row(ui, format!("{}   p. {}", row.title, row.page + 1))
    })
    .inner
}

/// Full-width single-line label that truncates instead of wrapping.
fn click_row(ui: &mut egui::Ui, text: String) -> egui::Response {
    ui.add(
        egui::Label::new(egui::RichText::new(text))
            .truncate()
            .sense(egui::Sense::click()),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
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
    let dim_border = egui::Stroke::new(1.0_f32, fg.gamma_multiply(0.25));
    let accent_stroke = egui::Stroke::new(1.0_f32, accent);

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
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, fg);
        widgets.noninteractive.bg_stroke = dim_border;
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, fg);
        widgets.inactive.weak_bg_fill = color_of(theme.panel_bg);
        widgets.inactive.bg_stroke = dim_border;
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, fg);
        widgets.hovered.weak_bg_fill = selection;
        widgets.hovered.bg_stroke = accent_stroke;
        widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, fg);
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
                    ui.colored_label(ERROR_RED, error);
                    if ui.small_button("dismiss").clicked() {
                        self.error = None;
                    }
                });
                ui.separator();
            }
            if self.editor.is_some() {
                self.show_theme_editor(ui);
            } else if self.document.is_some() {
                self.show_canvas(ui);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open a PDF to start reading (Ctrl+O).");
                });
            }
        });

        self.about_window(ctx);
        self.handle_input(ctx);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin_theme(name: &str) -> Theme {
        builtin(name).unwrap_or_else(|| panic!("{name} must exist"))
    }

    #[test]
    fn open_seeds_buffer_from_the_active_theme() {
        let theme = builtin_theme("Sepia");
        let editor = ThemeEditor::open(&theme);
        assert_eq!(editor.error, None);
        assert_eq!(parse(&editor.buffer), Ok(theme));
    }

    #[test]
    fn valid_edit_returns_the_parsed_theme() {
        let theme = builtin_theme("Light");
        let mut editor = ThemeEditor::open(&theme);
        let applied = editor
            .edit(to_yaml(&theme).replacen("name: Light", "name: Custom", 1))
            .expect("valid buffer applies");
        assert_eq!(applied.name, "Custom");
        assert_eq!(applied.page_bg, theme.page_bg);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn color_edit_applies_without_a_name_change() {
        let theme = builtin_theme("Dark");
        let mut editor = ThemeEditor::open(&theme);
        let applied = editor
            .edit(to_yaml(&theme).replace("accent: \"#4C8DF6\"", "accent: \"#FF0000\""))
            .expect("valid buffer applies");
        assert_eq!(applied.accent, Color::from([0xFF, 0x00, 0x00, 0xFF]));
    }

    #[test]
    fn invalid_edit_reports_none_and_keeps_the_error_verbatim() {
        let mut editor = ThemeEditor::open(&builtin_theme("Dark"));
        let applied = editor.edit("page_bg: oops\n".into());
        assert!(applied.is_none());
        let err = editor.error.as_deref().expect("error recorded");
        assert!(err.contains("invalid"), "{err}");
    }

    #[test]
    fn good_edit_after_a_bad_one_recovers() {
        let theme = builtin_theme("Warm Dark");
        let mut editor = ThemeEditor::open(&theme);
        assert!(editor.edit("name: [unclosed".into()).is_none());
        assert!(editor.error.is_some());
        let applied = editor.edit(to_yaml(&theme)).expect("recovers");
        assert_eq!(applied, theme);
        assert_eq!(editor.error, None);
    }
}
