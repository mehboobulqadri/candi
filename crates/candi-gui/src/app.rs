// SPDX-License-Identifier: AGPL-3.0

//! GUI reader shell: paged canvas (continuous, single-page, or dual-page
//! flow) over a background render pipeline, chrome (top bar / sidebar /
//! bottom bar), built-in theming, and session persistence via `candi-cli`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::{Range, RangeInclusive};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use candi_cli::{open_session, save_session};
use candi_core::{
    DEFAULT_THEME, Prefs, SessionState, ZoomMode, config_dir, config_path, load_prefs, store_prefs,
    write_file_atomically,
};
use candi_pdf::{BackendKind, Document, PageImage};
use candi_theme::{
    BUILTIN_NAMES, Color, Theme, builtin, canvas_bg, parse, recolor, retint, to_yaml,
};
use eframe::egui;
use egui::Key;

use crate::highlight::yaml_job;
use crate::icons::{Icon, IconRender};
use crate::keybinds::{Action, Keybinds};
use crate::render::cache::{
    CacheKey, DEFAULT_BUDGET_BYTES, ImageCache, RECOLORED_BUDGET_BYTES, RecolorKey,
};
use crate::render::layout::{self, Flow, GAP, Layout};
use crate::render::pipeline::{Pipeline, RenderRequest, RenderResult, panic_message};
use crate::search::SearchJob;
use crate::sidebar::{SearchHit, SidebarSection, TocRow, active_toc_row, date_only, flatten_toc};

/// Pages kept as textures around the current page; texture memory outside this
/// window is released while the original bitmaps stay cached.
const TEXTURE_KEEP_AROUND: usize = 6;
/// Upper end of the zoom slider; keyboard steps may still go higher, up to
/// candi-core's own limit (design spec §9).
const SLIDER_MAX_PERCENT: u16 = 400;
/// Corner rounding shared by page shadow, placeholder fill, and border.
const PAGE_ROUNDING: f32 = 3.0;
/// Sidebar contents indentation per outline nesting level.
const INDENT_PER_LEVEL: f32 = 12.0;
const ERROR_RED: egui::Color32 = egui::Color32::from_rgb(0xE5, 0x48, 0x4D);
/// Muted placeholder shown by doc-scoped surfaces (info window, sidebar
/// sections) while no document is open.
const NO_DOC: &str = "No document loaded";
/// Accent choices in the appearance panel, as RGB bytes.
const ACCENT_SWATCHES: [[u8; 3]; 8] = [
    [0x7C, 0x5C, 0xFF],
    [0x4C, 0x8D, 0xF6],
    [0xE5, 0x48, 0x4D],
    [0xF7, 0x6B, 0x15],
    [0xFF, 0xB2, 0x24],
    [0x46, 0xA7, 0x58],
    [0x12, 0xA5, 0x94],
    [0xE9, 0x3D, 0x82],
];
/// Bounds of the UI text-size slider.
const UI_SCALE_RANGE: RangeInclusive<f32> = 0.80..=1.40;
/// Bounds of the theme-editor font zoom (Ctrl+= / Ctrl+- over the editor).
const EDITOR_FONT_RANGE: RangeInclusive<f32> = 8.0..=40.0;
/// Multiplicative step of one editor font-zoom key press.
const EDITOR_FONT_STEP: f32 = 1.1;
const SWATCH_SIZE: f32 = 22.0;
/// Per-page render retry backoff by failure count: the first three failures
/// schedule an automatic retry, the fourth leaves the page terminal until
/// the reader clicks it.
const RETRY_BACKOFFS: [Duration; 3] = [
    Duration::from_millis(250),
    Duration::from_millis(1_000),
    Duration::from_millis(4_000),
];
/// Page toast: full opacity for this long after the last page change,
const TOAST_HOLD: Duration = Duration::from_millis(700);
/// then a linear fade over this long.
const TOAST_FADE: Duration = Duration::from_millis(200);
/// Height of the bottom chrome bar; the page toast anchors just above it.
const BOTTOM_BAR_HEIGHT: f32 = 42.0;
/// Banner when the reading-position sidecar failed to parse: the fresh
/// session must not silently replace the unreadable file, so saves stay
/// blocked until the reader engages with the document.
const SESSION_CORRUPT_BANNER: &str = "Reading-position file corrupt — started fresh (your previous file is preserved until you navigate)";
/// Custom theme files larger than this are rejected before reading.
const MAX_THEME_BYTES: u64 = 256 * 1024;

/// A theme file above the cap is rejected unread: a pathological file must
/// not be slurped into a String on startup.
fn oversized_theme(size: u64) -> bool {
    size > MAX_THEME_BYTES
}

/// Everything a finished open hands the UI thread: the loaded session and
/// document plus the once-per-open page sizes and flattened outline, all
/// computed on the worker so a 1556-page sweep never blocks input.
struct OpenedPayload {
    document: Arc<dyn Document>,
    session: SessionState,
    warning: Option<String>,
    page_sizes: Result<Vec<(f32, f32)>, String>,
    toc_rows: Result<Vec<TocRow>, String>,
}

/// One in-flight open: the payload channel, the job's target path (for the
/// opening notice), and the flag a newer open sets so this one's page-size
/// sweep stops early. Its results are dead the moment a newer job exists —
/// the UI dropped this receiver when it started the newer open.
struct OpenJob {
    path: PathBuf,
    rx: Receiver<Result<OpenedPayload, String>>,
    cancel: Arc<AtomicBool>,
}

/// Worker half of an open: session and document, then page sizes and
/// outline. A superseded job's results are never delivered — the UI dropped
/// the receiver when it started the newer open — so the cancel path's error
/// text is a placeholder by construction.
fn compute_open_payload(
    path: &Path,
    backend: BackendKind,
    cancel: &AtomicBool,
) -> Result<OpenedPayload, String> {
    let opened = open_session(path, backend).map_err(|err| err.to_string())?;
    if cancel.load(Ordering::Relaxed) {
        return Err("superseded".to_owned());
    }
    let document: Arc<dyn Document> = Arc::from(opened.document);
    let count = document.page_count();
    let mut sizes = Vec::with_capacity(count);
    for page in 0..count {
        if cancel.load(Ordering::Relaxed) {
            return Err("superseded".to_owned());
        }
        match document.page_size(page) {
            Ok(size) => sizes.push(size),
            Err(err) => return Err(format!("page {}: {err}", page + 1)),
        }
    }
    let toc_rows = match document.outline() {
        Ok(items) => Ok(flatten_toc(&items)),
        Err(err) => Err(format!("table of contents: {err}")),
    };
    Ok(OpenedPayload {
        document,
        session: opened.session,
        warning: opened.warning,
        page_sizes: Ok(sizes),
        toc_rows,
    })
}

/// Per-page render failure ledger entry: failures so far at the current
/// scale, when the next automatic retry fires — `None` once the backoffs
/// are exhausted (terminal; the page shows click-to-retry) — and the last
/// error detail, surfaced as hover text on that state.
#[derive(Debug, Clone, PartialEq)]
struct FailState {
    attempt: usize,
    next_retry: Option<Instant>,
    detail: String,
}

/// Record a render failure for `key`; the retry schedule follows
/// [`RETRY_BACKOFFS`], ending in the terminal state.
fn note_render_failure(
    failed: &mut HashMap<CacheKey, FailState>,
    key: CacheKey,
    now: Instant,
    detail: String,
) {
    let attempt = failed.get(&key).map_or(0, |state| state.attempt) + 1;
    let next_retry = RETRY_BACKOFFS
        .get(attempt - 1)
        .map(|backoff| now + *backoff);
    failed.insert(
        key,
        FailState {
            attempt,
            next_retry,
            detail,
        },
    );
}

/// Whether a page needs a render request right now: never queued, or a
/// failure whose retry backoff has elapsed. Terminal failures and futures
/// backoffs wait; the clock comes in as a parameter so the schedule is
/// testable.
fn wants_render(
    pending: &HashSet<CacheKey>,
    failed: &HashMap<CacheKey, FailState>,
    key: CacheKey,
    now: Instant,
) -> bool {
    if pending.contains(&key) {
        return false;
    }
    match failed.get(&key) {
        None => true,
        Some(FailState {
            next_retry: Some(due),
            ..
        }) => *due <= now,
        Some(FailState {
            next_retry: None, ..
        }) => false,
    }
}

/// Zoom changed: forget failures (they were scale-specific) and drop
/// pending entries queued at other scales — their results are superseded
/// and their keys would otherwise leak in the queue forever.
fn prune_stale_scale(
    pending: &mut HashSet<CacheKey>,
    failed: &mut HashMap<CacheKey, FailState>,
    scale_q: u16,
) {
    failed.clear();
    pending.retain(|key| key.scale_q == scale_q);
}

/// Outcome of offering a batch of renders to the pipeline.
enum Submission {
    /// Queued on the worker; the keys were recorded as pending.
    Queued,
    /// No worker — the renderer already stopped and its banner stands.
    NoRenderer,
    /// The worker refused the batch; it just stopped.
    Refused,
}

/// Submit `wanted` and record the keys as pending only after an accepted
/// submit, so a dead worker can never leave phantom queue entries behind
/// that nothing will ever drain.
fn queue_renders(
    pipeline: Option<&Pipeline>,
    wanted: &[RenderRequest],
    pending: &mut HashSet<CacheKey>,
) -> Submission {
    let Some(pipeline) = pipeline else {
        return Submission::NoRenderer;
    };
    if !pipeline.submit(wanted) {
        return Submission::Refused;
    }
    pending.extend(wanted.iter().map(|req| req.key()));
    Submission::Queued
}

/// Opacity (1 → 0) of a toast last refreshed at `shown`: full while the
/// hold lasts, then a linear fade to zero.
fn toast_opacity(shown: Instant, now: Instant) -> f32 {
    let elapsed = now.saturating_duration_since(shown);
    if elapsed < TOAST_HOLD {
        return 1.0;
    }
    let fade = elapsed - TOAST_HOLD;
    if fade >= TOAST_FADE {
        return 0.0;
    }
    1.0 - fade.as_secs_f32() / TOAST_FADE.as_secs_f32()
}

/// Per-frame registry of the frame's text-edit widgets, feeding the
/// focus-release decision: every editable field registers itself, and ones
/// that explicitly request focus mark the frame so the blur pass cannot
/// clobber a request with the very click that caused it.
#[derive(Default)]
struct FocusGuard {
    editables: Vec<(egui::Id, egui::Rect)>,
    requested: bool,
}

impl FocusGuard {
    fn register(&mut self, response: &egui::Response) {
        self.editables.push((response.id, response.rect));
    }

    fn request(&mut self, response: &egui::Response) {
        self.requested = true;
        response.request_focus();
    }
}

/// Whether a pointer interaction should release a focused text field: a
/// click or wheel scroll anywhere outside every editable field, while a
/// field holds focus and nothing requested focus this frame (a same-frame
/// request must not be clobbered by the click that triggered it).
fn should_release_focus(
    clicked: Option<egui::Pos2>,
    scrolled: Option<egui::Pos2>,
    focus: Option<egui::Id>,
    editables: &[(egui::Id, egui::Rect)],
    focus_requested: bool,
) -> bool {
    if focus_requested || focus.is_none() {
        return false;
    }
    let outside = |pos: Option<egui::Pos2>| {
        pos.is_some_and(|pos| !editables.iter().any(|(_, rect)| rect.contains(pos)))
    };
    outside(clicked) || outside(scrolled)
}

/// Whether the viewport-center page deserves a fresh page toast: only a
/// page different from the last toasted one. The marker survives toast
/// expiry, so hovering the same page never re-toasts it.
fn should_toast(last_toasted: Option<usize>, page: usize) -> bool {
    last_toasted != Some(page)
}

/// Focus egui's own Tab/arrow navigation parked on a chrome widget, if any:
/// the keybind dispatcher treats any keyboard focus as typing, so focus on
/// a button silently disables every plain-key binding. Only the app's
/// registered text-edit fields may hold focus across frames.
fn transient_focus(
    focused: Option<egui::Id>,
    editables: &[(egui::Id, egui::Rect)],
) -> Option<egui::Id> {
    focused.filter(|id| !editables.iter().any(|(editable, _)| editable == id))
}

/// Horizontal offset of the page toast from screen center: the zoom
/// slider's center, or 0 before the bottom bar's first frame. The slider
/// lives in the bar's middle third and the toast is capped at a third of
/// the screen, so the capped toast never leaves the window.
fn toast_offset(slider: Option<egui::Rect>, screen_center_x: f32) -> f32 {
    slider.map_or(0.0, |rect| rect.center().x - screen_center_x)
}

/// Cache-key mix of the recolor pass's two page colors: themes mapping to
/// the same page palette share recolorized bitmaps.
fn recolor_key(page_bg: Color, page_fg: Color) -> u64 {
    let pack = |c: Color| u32::from_be_bytes([c.r(), c.g(), c.b(), c.a()]);
    u64::from(pack(page_bg)) << 32 | u64::from(pack(page_fg))
}

/// The wait until the next timed repaint the schedulers need, `None` when
/// none does. An unfocused (or occluded) window schedules nothing: eframe
/// 0.30 never gates buffer swaps, so a background repaint tick keeps the
/// Wayland driver blocking the main thread until the compositor declares
/// the window unresponsive ("terminate or wait"). Workers keep completing
/// regardless — renders, searches, and opens apply on the next
/// event-driven frame, and egui-winit repaints on Focused(true), so
/// everything resumes the moment the window regains focus.
fn should_schedule_repaint(
    focused: bool,
    pending_renders: bool,
    next_retry: Option<Duration>,
    polling_job: bool,
    toast_wait: Option<Duration>,
) -> Option<Duration> {
    if !focused {
        return None;
    }
    const CADENCE: Duration = Duration::from_millis(50);
    let render_wait = if pending_renders {
        Some(CADENCE)
    } else {
        next_retry
    };
    [render_wait, polling_job.then_some(CADENCE), toast_wait]
        .into_iter()
        .flatten()
        .min()
}

/// A promoted GPU texture for one page plus the scale it was rendered at.
struct PageTexture {
    scale_q: u16,
    handle: egui::TextureHandle,
}

/// Center-pane theme editor state: the YAML buffer, its last parse outcome,
/// and the monospace font size (zoomable in place). Openness is encoded by
/// [`ReaderApp`] holding `Option<ThemeEditor>`; applying a parsed theme
/// stays with the caller so this stays egui-free and unit-testable.
struct ThemeEditor {
    buffer: String,
    error: Option<String>,
    /// "Save as…" input; reused across attempts.
    saved_name: String,
    /// Last "Save as…" outcome, inline until the next save attempt.
    saved: Option<String>,
    font_size: f32,
}

impl ThemeEditor {
    fn open(theme: &Theme) -> Self {
        Self {
            buffer: to_yaml(theme),
            error: None,
            saved_name: String::new(),
            saved: None,
            font_size: crate::highlight::FONT_SIZE,
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

/// Deferred relayout kind for [`ReaderApp::debug_relayout`] capture states.
enum DebugRelayout {
    Dual,
    Single,
    Percent,
}

pub struct ReaderApp {
    backend: BackendKind,
    path: Option<PathBuf>,
    filename: String,
    document: Option<Arc<dyn Document>>,
    session: SessionState,
    /// The sidecar failed to parse at open: automatic saves stay blocked
    /// (see [`Self::note_engagement`]) so the fresh session cannot destroy
    /// the unreadable file before the reader actually engages.
    session_corrupt: bool,
    theme: Theme,
    /// Name of the theme whose visuals were last pushed into egui.
    applied_theme: String,
    /// App config (theme choice + recents), persisted next to the binary's
    /// XDG config dir; `None` disables persistence (no HOME/XDG).
    config: Prefs,
    config_path: Option<PathBuf>,

    cache: ImageCache,
    /// Recolorized bitmaps keyed by page, scale, and page colors; survives
    /// theme switches so toggling back is a texture upload, not a re-map.
    recolored: ImageCache<RecolorKey>,
    pipeline: Option<Pipeline>,
    /// Render jobs queued on the worker, awaiting their result.
    pending: HashSet<CacheKey>,
    /// Per-page failure ledger at `failed_scale_q`: automatic retries with
    /// bounded backoff, then a terminal click-to-retry state.
    failed: HashMap<CacheKey, FailState>,
    failed_scale_q: u16,
    /// Promoted textures keyed by page; dropped on theme switches.
    textures: HashMap<usize, PageTexture>,
    /// Lucide icon texture cache.
    icons: IconRender,

    layout: Layout,
    /// `(avail_w, avail_h, zoom, flow)` the current layout was built for.
    layout_key: Option<(f32, f32, ZoomMode, Flow)>,
    /// Per-page `(width, height)` in points, fetched once at open.
    page_sizes: Vec<(f32, f32)>,
    /// Effective quantized zoom percent; resolved from fit-width on relayout.
    zoom_pct: u16,
    /// Fit-page is active; the percent zoom is re-derived on every relayout
    /// so window resizes keep the page fully visible. A manual zoom clears it.
    fit_page: bool,
    /// Page flow (continuous / 1-up / 2-up); session-local, not persisted.
    flow: Flow,
    /// Content-y under the viewport center from the last painted canvas
    /// frame; the reference point for relayout anchors.
    canvas_center_y: f32,
    /// Relayout anchor — page and depth-fraction to keep under the viewport
    /// center after the next zoom or flow rebuild.
    scroll_anchor: Option<(usize, f32)>,
    /// Capture scaffolding: a flow or zoom relayout deferred to after the
    /// first painted frame, so anchored-relayout evidence needs no input
    /// injection (`CANDI_UI_DEBUG=delay-dual|delay-single|delay-percent`).
    debug_relayout: Option<DebugRelayout>,

    /// Capture scaffolding: the theme picker popup is forced open on the
    /// next frame (`CANDI_UI_DEBUG=theme-picker`), since egui popups cannot
    /// be driven without input injection.
    debug_picker: bool,

    sidebar_open: bool,
    section: SidebarSection,
    /// Text-edit widgets painted this frame, for the focus-release pass.
    focus: FocusGuard,
    /// Toast showing the viewport-center page: (page, last change).
    toast: Option<(usize, Instant)>,
    /// Last painted screen rect of the zoom slider; the page toast anchors
    /// above it. `None` until the bottom bar's first frame.
    zoom_slider_rect: Option<egui::Rect>,
    /// Last page a toast fired for, remembered across toast expiry so
    /// hovering the same page never re-toasts it.
    last_toasted_page: Option<usize>,
    /// Last TOC row clicked with the page it pointed at; preferred over the
    /// computed accent until the reader leaves that page.
    toc_follow: Option<(usize, usize)>,
    /// Slider-driven UI text scale on top of [`Self::base_ppp`].
    ui_scale: f32,
    /// Native pixels-per-point captured at startup; [`Self::ui_scale`]
    /// multiplies it.
    base_ppp: f32,
    focus_search: bool,
    /// Focus mode (design spec §8): chrome hidden, document only.
    focus_mode: bool,
    /// Inline page-jump buffer; `Some` while the counter is an input field.
    page_jump: Option<String>,
    page_jump_focus: bool,
    /// Page of the bookmark being renamed inline; `Some` while editing.
    renaming: Option<usize>,
    rename_buffer: String,
    rename_focus: bool,
    /// Last jump attempt failed validation; tints the input until edited.
    jump_invalid: bool,
    search_query: String,
    search_hits: Option<Vec<SearchHit>>,
    /// Running full-document scan; results stream into `search_hits`.
    search_job: Option<SearchJob>,
    /// In-flight document open; `Some` until its payload applies or fails.
    /// Superseded by any newer open.
    open_job: Option<OpenJob>,
    /// An inline text field (page jump, bookmark rename) handled Escape
    /// this frame; the keybind dispatcher must not also read it as
    /// CloseOverlay.
    escape_consumed: bool,
    /// In-flight native file picker; `Some` while the dialog is open, so
    /// requests stay single-flight and the UI thread never blocks.
    dialog_result: Option<Receiver<Option<PathBuf>>>,
    /// Hit rects of one painted page: (page, lowercase query, [x0, y0, x1,
    /// y1] points top-left origin). Refreshed lazily per visible page.
    highlight: Option<(usize, String, Vec<[f32; 4]>)>,
    /// Flattened table of contents, loaded once per open; empty = none.
    toc_rows: Vec<TocRow>,
    about_open: bool,
    info_open: bool,
    shortcuts_open: bool,
    /// Case-insensitive filter applied to the shortcuts window's rows.
    shortcut_filter: String,
    /// User-editable key → action map, loaded (and seeded on first run)
    /// from `keybinds.json` next to the app config.
    keybinds: Keybinds,
    /// Custom themes loaded from `<config>/themes/*.yaml`, sorted by name;
    /// each entry keeps its backing file for deletion.
    custom_themes: Vec<(Theme, PathBuf)>,
    /// Inline status for theme-registry actions (delete failures); shown
    /// under the My Themes list until the next action replaces it.
    theme_status: Option<String>,
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
        cc: &eframe::CreationContext<'_>,
        initial: Option<PathBuf>,
        backend: BackendKind,
    ) -> Self {
        // Inter as the UI face (mockup typography); the semibold weight sits
        // second in the family so strong text can pick it up.
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "inter".into(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")).into(),
        );
        fonts.font_data.insert(
            "inter-semibold".into(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-SemiBold.ttf"))
                .into(),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "inter".into());
            family.insert(1, "inter-semibold".into());
        }
        cc.egui_ctx.set_fonts(fonts);
        let config_path = config_path();
        let config = config_path
            .as_deref()
            .map(|path| {
                let mut prefs = load_prefs(path);
                // Prune entries whose files have vanished since the last run.
                prefs.recents.retain(|recent| recent.path.is_file());
                prefs
            })
            .unwrap_or_default();
        // Custom themes load before the persisted name resolves, so a
        // config pointing at a custom theme picks it up directly.
        let custom_themes = Self::load_custom_themes(themes_dir().as_deref());
        let mut app = Self {
            backend,
            path: None,
            filename: String::new(),
            document: None,
            session: SessionState::new(1),
            session_corrupt: false,
            theme: Self::startup_theme(&config, &custom_themes),
            applied_theme: String::new(),
            config,
            config_path,
            cache: ImageCache::new(DEFAULT_BUDGET_BYTES),
            recolored: ImageCache::new(RECOLORED_BUDGET_BYTES),
            pipeline: None,
            pending: HashSet::new(),
            failed: HashMap::new(),
            failed_scale_q: 0,
            textures: HashMap::new(),
            icons: IconRender::default(),
            layout: Layout::default(),
            layout_key: None,
            page_sizes: Vec::new(),
            zoom_pct: layout::MIN_ZOOM_PERCENT,
            fit_page: false,
            flow: Flow::Continuous,
            canvas_center_y: 0.0,
            scroll_anchor: None,
            debug_relayout: None,
            debug_picker: false,
            sidebar_open: false,
            section: SidebarSection::Contents,
            focus: FocusGuard::default(),
            toast: None,
            zoom_slider_rect: None,
            last_toasted_page: None,
            toc_follow: None,
            ui_scale: 1.0,
            base_ppp: cc.egui_ctx.pixels_per_point(),
            focus_search: false,
            focus_mode: false,
            page_jump: None,
            page_jump_focus: false,
            renaming: None,
            rename_buffer: String::new(),
            rename_focus: false,
            jump_invalid: false,
            search_query: String::new(),
            search_hits: None,
            search_job: None,
            open_job: None,
            escape_consumed: false,
            dialog_result: None,
            highlight: None,
            toc_rows: Vec::new(),
            about_open: false,
            info_open: false,
            shortcuts_open: false,
            shortcut_filter: String::new(),
            keybinds: Keybinds::load_or_init(config_dir().as_deref()),
            custom_themes,
            theme_status: None,
            editor: None,
            restore_frac: None,
            pending_scroll: None,
            primed: true,
            error: None,
        };
        if let Some(path) = initial {
            app.open_path(path);
            // The startup open stays synchronous: capture states and the
            // first frame need the document ready before the loop begins.
            while app.open_job.is_some() {
                app.poll_open();
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        if let Ok(mode) = std::env::var("CANDI_UI_DEBUG") {
            // Capture scaffolding: pre-opens the UI state each mode names so
            // scripts/shot.sh-style evidence needs no input injection.
            app.sidebar_open = mode != "nosidebar";
            match mode.as_str() {
                "search" => {
                    app.section = SidebarSection::Search;
                    app.search_query = "attention".into();
                    // Capture scaffolding: run the scan to completion before
                    // the first frame so the shot shows the final rows.
                    app.start_search();
                    while app.search_job.is_some() {
                        app.poll_search();
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
                "search-empty" => app.section = SidebarSection::Search,
                "bookmarks" => app.section = SidebarSection::Bookmarks,
                "appearance" => app.section = SidebarSection::Appearance,
                "theme-picker" => {
                    app.section = SidebarSection::Appearance;
                    app.debug_picker = true;
                }
                "delete-active-theme" => {
                    // Capture scaffolding: deletes the first custom theme
                    // through the real delete path. If it is also the
                    // persisted theme, the safe Dark fallback is visible
                    // without input injection.
                    if let Some((theme, _)) = app.custom_themes.first() {
                        let name = theme.name.clone();
                        app.delete_custom_theme(&name);
                    }
                    app.section = SidebarSection::Appearance;
                    app.debug_picker = true;
                }
                "accent-purple" => {
                    app.section = SidebarSection::Appearance;
                    app.set_accent(&cc.egui_ctx, ACCENT_SWATCHES[0]);
                }
                "editor" => {
                    app.sidebar_open = false;
                    app.open_theme_editor();
                }
                "editor-zoom" => {
                    app.sidebar_open = false;
                    app.open_theme_editor();
                    if let Some(editor) = app.editor.as_mut() {
                        editor.font_size = 20.0;
                    }
                }
                "editor-save" => {
                    // Capture scaffolding: seeds the editor buffer with a
                    // tweaked Sepia and runs the Save-as path, so captures
                    // show the real post-save state without input injection.
                    app.sidebar_open = false;
                    app.open_theme_editor();
                    let seeded = to_yaml(&builtin("Sepia").expect("built-in theme parses"))
                        .replace("#C89B3C", "#D2691E");
                    if let Some(editor) = app.editor.as_mut() {
                        editor.buffer = seeded;
                        editor.saved_name = "Sepia Plus".into();
                    }
                    app.save_editor_theme_as("Sepia Plus");
                }
                "shortcuts" => {
                    app.shortcuts_open = true;
                    app.shortcut_filter = "page".into();
                }
                "shortcuts-all" => app.shortcuts_open = true,
                "info" => app.info_open = true,
                "deep" | "deep-dual" | "deep-single" | "deep-percent" => {
                    // Mid-document capture states: page 8, then the flow or
                    // zoom change whose anchored relayout is under test.
                    if app.document.is_some() {
                        app.goto_page(7, None);
                        match mode.as_str() {
                            "deep-dual" => app.set_flow(Flow::Dual),
                            "deep-single" => app.set_flow(Flow::Single),
                            "deep-percent" => app.set_zoom_percent(200.0),
                            _ => {}
                        }
                    }
                }
                "delay-dual" | "delay-single" | "delay-percent" if app.document.is_some() => {
                    app.goto_page(7, None);
                    app.debug_relayout = Some(match mode.as_str() {
                        "delay-dual" => DebugRelayout::Dual,
                        "delay-single" => DebugRelayout::Single,
                        _ => DebugRelayout::Percent,
                    });
                }
                _ => {}
            }
        }
        app
    }

    /// Start opening `path` on a worker thread: the session load, the
    /// once-per-open page-size sweep (a per-page loop that blocks the UI
    /// thread for seconds on large documents), and the outline all compute
    /// off-thread. A newer open supersedes the running one; the previous
    /// document stays visible until a payload applies (the welcome screen
    /// holds for a first open), and a failed open preserves it.
    fn open_path(&mut self, path: PathBuf) {
        self.error = None;
        self.cancel_search();
        if let Some(job) = self.open_job.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let backend = self.backend;
        let job_path = path.clone();
        std::thread::Builder::new()
            .name("candi-open".into())
            .spawn(move || {
                let payload = catch_unwind(AssertUnwindSafe(|| {
                    compute_open_payload(&job_path, backend, &flag)
                }))
                .unwrap_or_else(|panic| Err(panic_message(&panic, "open")));
                let _ = tx.send(payload);
            })
            .expect("spawn candi-open worker thread");
        self.open_job = Some(OpenJob { path, rx, cancel });
    }

    /// Apply a finished open job, if any: success replaces the document and
    /// every doc-scoped surface, failure keeps the previous document
    /// exactly as it was and shows the usual banner.
    fn poll_open(&mut self) {
        let Some(job) = self.open_job.take() else {
            return;
        };
        let outcome = match job.rx.try_recv() {
            Ok(outcome) => outcome,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.open_job = Some(job);
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("document open failed".to_owned())
            }
        };
        match outcome {
            Ok(payload) => self.apply_opened(job.path.clone(), payload),
            Err(err) => self.error = Some(err),
        }
    }

    /// Rebuild every doc-scoped surface from a finished open. A successful
    /// open replaces the previous document wholesale; this never runs for a
    /// failed one.
    fn apply_opened(&mut self, path: PathBuf, payload: OpenedPayload) {
        self.search_query.clear();
        self.highlight = None;
        self.renaming = None;
        self.rename_buffer.clear();
        self.rename_focus = false;
        self.toc_rows.clear();
        self.cache = ImageCache::new(DEFAULT_BUDGET_BYTES);
        self.recolored = ImageCache::new(RECOLORED_BUDGET_BYTES);
        self.pending.clear();
        self.failed.clear();
        self.failed_scale_q = 0;
        self.textures.clear();
        self.pipeline = None;
        self.layout_key = None;
        self.page_sizes.clear();
        self.pending_scroll = None;
        self.scroll_anchor = None;
        self.canvas_center_y = 0.0;
        self.primed = false;
        self.sidebar_open = true;
        self.fit_page = false;
        self.flow = Flow::Continuous;
        self.ui_scale = 1.0;
        self.toast = None;
        self.last_toasted_page = None;
        self.toc_follow = None;

        let theme_name = payload.session.theme.clone();
        self.session = payload.session;
        self.session_corrupt = payload.warning.is_some();
        if self.session_corrupt {
            self.error = Some(SESSION_CORRUPT_BANNER.to_owned());
        }
        self.set_theme(&theme_name);
        self.path = Some(path.clone());
        self.filename = filename_of(&path);
        self.config.record_open(&path);
        self.config.onboarding_done = true;
        self.save_config();
        self.pipeline = Some(Pipeline::spawn(payload.document.clone()));
        self.document = Some(payload.document);
        match payload.page_sizes {
            Ok(sizes) => self.page_sizes = sizes,
            Err(err) => self.error = Some(err),
        }
        match payload.toc_rows {
            Ok(rows) => self.toc_rows = rows,
            Err(err) => self.error = Some(err),
        }
        if let ZoomMode::Percent(p) = self.session.zoom {
            self.zoom_pct = p;
        }
        self.restore_frac = Some(self.session.scroll_frac);
    }

    /// Theme the app starts with: the persisted config choice (built-in or
    /// custom), falling back to the built-in default when the name is unknown.
    fn startup_theme(config: &Prefs, customs: &[(Theme, PathBuf)]) -> Theme {
        Self::resolve_theme(&config.theme, customs)
            .unwrap_or_else(|| builtin(DEFAULT_THEME).expect("built-in default theme parses"))
    }

    /// Look up a theme by name among built-ins and custom files; built-ins
    /// win over same-named customs.
    fn resolve_theme(name: &str, customs: &[(Theme, PathBuf)]) -> Option<Theme> {
        builtin(name).or_else(|| {
            customs
                .iter()
                .find(|(theme, _)| theme.name == name)
                .map(|(theme, _)| theme.clone())
        })
    }

    /// Load every `themes/*.yaml` whose embedded name matches its file stem,
    /// sorted by name; unreadable, unparsable, or misnamed files warn and
    /// are skipped.
    fn load_custom_themes(dir: Option<&Path>) -> Vec<(Theme, PathBuf)> {
        let Some(dir) = dir else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        paths.sort_unstable();
        paths
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("yaml")))
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?.to_owned();
                if let Ok(meta) = fs::metadata(&path)
                    && oversized_theme(meta.len())
                {
                    eprintln!(
                        "candi: skipping custom theme {}: larger than {} KiB",
                        path.display(),
                        MAX_THEME_BYTES / 1024
                    );
                    return None;
                }
                let source = match fs::read_to_string(&path) {
                    Ok(source) => source,
                    Err(err) => {
                        eprintln!("candi: reading custom theme {}: {err}", path.display());
                        return None;
                    }
                };
                match parse(&source) {
                    Ok(theme) if theme.name == stem => Some((theme, path)),
                    Ok(theme) => {
                        eprintln!(
                            "candi: skipping custom theme {}: embedded name {:?} does not match file stem {:?}",
                            path.display(),
                            theme.name,
                            stem
                        );
                        None
                    }
                    Err(err) => {
                        eprintln!("candi: skipping custom theme {}: {err}", path.display());
                        None
                    }
                }
            })
            .collect()
    }

    /// Persist the app config (theme choice, recents); a failed write warns
    /// instead of disturbing the session.
    fn save_config(&mut self) {
        let Some(path) = self.config_path.as_deref() else {
            return;
        };
        if let Err(err) = store_prefs(path, &self.config) {
            eprintln!("candi: saving config: {err}");
        }
    }

    /// Mark the quick-start panel done and persist immediately; the welcome
    /// screen stays plain forever after.
    fn dismiss_onboarding(&mut self) {
        self.config.onboarding_done = true;
        self.save_config();
    }

    /// Switch the active theme — built-in or custom; unknown names fall
    /// back to Dark. Visuals are re-applied at the top of the next
    /// [`ReaderApp::update`]; texture slots are dropped so pages re-promote
    /// from the theme cache (or, on first sight, their cached originals).
    /// Closes the theme editor: a dropdown/menu switch means the buffer is
    /// no longer authoritative. The choice persists to the app config
    /// immediately.
    fn set_theme(&mut self, name: &str) {
        self.theme = Self::resolve_theme(name, &self.custom_themes)
            .unwrap_or_else(|| builtin(DEFAULT_THEME).expect("default theme parses"));
        self.session.theme = self.theme.name.clone();
        self.config.theme = self.theme.name.clone();
        self.save_config();
        self.textures.clear();
        self.editor = None;
    }

    /// Delete a custom theme's file and registry entry; deleting the active
    /// theme falls back to Dark safely via [`Self::set_theme`]. A failed
    /// deletion keeps the entry and reports inline — the theme must stay
    /// usable when only its backing file is gone.
    fn delete_custom_theme(&mut self, name: &str) {
        let Some((_, path)) = self
            .custom_themes
            .iter()
            .find(|(theme, _)| theme.name == name)
        else {
            return;
        };
        match fs::remove_file(path) {
            Ok(()) => {
                self.custom_themes.retain(|(theme, _)| theme.name != name);
                if self.theme.name == name {
                    self.set_theme(DEFAULT_THEME);
                }
                self.theme_status = None;
            }
            Err(err) => {
                self.theme_status = Some(format!("Deleting {name:?} failed: {err}"));
            }
        }
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
        self.config.theme = self.theme.name.clone();
        self.save_config();
        self.textures.clear();
        self.applied_theme.clear();
    }

    /// Save the editor buffer as `themes/<name>.yaml`; the saved theme
    /// becomes active and its canonical YAML re-seeds the buffer so the
    /// embedded `name:` matches. The inline status reports each outcome.
    fn save_editor_theme_as(&mut self, raw_name: &str) {
        let Some(name) = sanitize_theme_name(raw_name.trim()) else {
            if let Some(editor) = self.editor.as_mut() {
                editor.saved = Some(format!(
                    "{raw_name:?}: use letters, digits, '-', '_' or spaces."
                ));
            }
            return;
        };
        let outcome = match self.editor.as_mut() {
            None => None,
            Some(editor) => match parse(&editor.buffer) {
                Err(_) => {
                    editor.saved = Some("Fix the YAML errors above before saving.".to_owned());
                    None
                }
                Ok(mut theme) => {
                    theme.name = name.clone();
                    Some((to_yaml(&theme), theme))
                }
            },
        };
        let Some((body, theme)) = outcome else {
            return;
        };
        let Some(dir) = themes_dir() else {
            if let Some(editor) = self.editor.as_mut() {
                editor.saved =
                    Some("No XDG_CONFIG_HOME/HOME — nowhere to save a custom theme.".to_owned());
            }
            return;
        };
        let path = dir.join(format!("{name}.yaml"));
        let write = fs::create_dir_all(&dir).and_then(|()| write_file_atomically(&path, &body));
        if let Err(err) = write {
            if let Some(editor) = self.editor.as_mut() {
                editor.saved = Some(format!("Saving failed: {err}"));
            }
            return;
        }
        if let Some(editor) = self.editor.as_mut() {
            editor.buffer.clone_from(&body);
            editor.saved = Some(format!("Saved {}", path.display()));
        }
        match self
            .custom_themes
            .iter_mut()
            .find(|(existing, _)| existing.name == name)
        {
            Some(slot) => slot.0 = theme.clone(),
            None => self.custom_themes.push((theme.clone(), path)),
        }
        self.custom_themes
            .sort_unstable_by_key(|(theme, _)| theme.name.to_lowercase());
        self.apply_edited_theme(theme);
    }

    /// Session-local accent override (appearance swatches): re-applies the
    /// visuals immediately, retinting the chrome toward the new accent even
    /// though the theme name — the cache key in `applied_theme` — is
    /// unchanged.
    fn set_accent(&mut self, ctx: &egui::Context, rgb: [u8; 3]) {
        self.theme.accent = Color::from([rgb[0], rgb[1], rgb[2], 0xFF]);
        apply_theme(ctx, &self.theme);
        self.applied_theme.clone_from(&self.theme.name);
    }

    fn page_count(&self) -> usize {
        self.document.as_ref().map_or(0, |doc| doc.page_count())
    }

    /// Genuine reader engagement — navigating, bookmarking, zooming, or
    /// switching the flow. Authorizes overwriting a corrupt sidecar on
    /// later automatic saves; until then the fresh session is never
    /// written over the unreadable file.
    fn note_engagement(&mut self) {
        self.session_corrupt = false;
    }

    fn goto_page(&mut self, page: usize, dest_top: Option<f32>) {
        let count = self.page_count();
        if count == 0 {
            return;
        }
        self.note_engagement();
        let page = page.min(count - 1);
        self.session.page = page;
        // An explicit jump is more specific than any pending relayout anchor.
        self.scroll_anchor = None;
        if let Some(rect) = self.layout.rects.get(page) {
            let mut y = (rect.y - GAP).max(0.0);
            if let Some(dest) = dest_top {
                // The destination lands `dest` points below the page top —
                // `dest * zoom` in content coordinates, kept inside the page.
                y += (dest * self.layout.zoom).clamp(0.0, rect.h);
            }
            self.pending_scroll = Some(y);
        }
    }

    /// Pin the viewport-center content point so the pending relayout keeps
    /// the page under the center in place instead of sliding away.
    fn record_center_anchor(&mut self) {
        if let Some(anchor) = center_anchor(&self.layout, self.canvas_center_y) {
            self.scroll_anchor = Some(anchor);
        }
    }

    /// Switch to percent zoom at `percent`, keeping the viewport-center
    /// content pinned across the relayout.
    fn set_zoom_percent(&mut self, percent: f32) {
        self.note_engagement();
        self.fit_page = false;
        self.record_center_anchor();
        self.session.zoom = ZoomMode::Percent(layout::quantize_nearest(percent));
    }

    fn zoom_step(&mut self, delta_percent: i16) {
        self.set_zoom_percent(f32::from(self.zoom_pct) + f32::from(delta_percent));
    }

    /// Leave any percent zoom and refit the document to the window width.
    fn zoom_fit_width(&mut self) {
        self.note_engagement();
        self.fit_page = false;
        self.record_center_anchor();
        self.session.zoom = ZoomMode::FitWidth;
    }

    /// Switch the page flow; each flow refits to its widest row so spreads
    /// always land fully visible. The anchor page snaps to the destination
    /// flow's row-first page — spread pages share one row band, so the
    /// offset is identical while toggling back keeps the same primary page.
    fn set_flow(&mut self, flow: Flow) {
        self.note_engagement();
        self.record_center_anchor();
        if let Some((page, _)) = self.scroll_anchor.as_mut() {
            *page -= *page % layout::pages_per_row(flow);
        }
        self.flow = flow;
        self.fit_page = false;
        self.session.zoom = ZoomMode::FitWidth;
    }

    /// Advance through the built-in themes in cycling order.
    fn cycle_theme(&mut self) {
        let next = BUILTIN_NAMES
            .iter()
            .position(|name| *name == self.theme.name)
            .map_or(0, |idx| (idx + 1) % BUILTIN_NAMES.len());
        self.set_theme(BUILTIN_NAMES[next]);
    }

    fn save_state(&mut self) {
        if self.session_corrupt {
            return;
        }
        let Some(path) = self.path.as_deref() else {
            return;
        };
        if let Err(err) = save_session(path, &self.session) {
            self.error = Some(format!("saving state: {err}"));
        }
    }

    /// Offer a native PDF picker without blocking the UI thread: the modal
    /// runs on its own thread and reports through a channel that the frame
    /// loop polls. Requests while one is already open are ignored.
    fn open_dialog(&mut self) {
        if self.dialog_result.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let dialog = crate::pdf_dialog();
        std::thread::Builder::new()
            .name("candi-dialog".into())
            .spawn(move || {
                let _ = tx.send(dialog.pick_file());
            })
            .expect("spawn candi-dialog thread");
        self.dialog_result = Some(rx);
    }

    /// Apply a finished file-picker result, if any.
    fn poll_dialog(&mut self) {
        let mut picked = None;
        if let Some(rx) = self.dialog_result.as_ref() {
            match rx.try_recv() {
                Ok(result) => picked = Some(result),
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
            }
        } else {
            return;
        }
        self.dialog_result = None;
        if let Some(path) = picked.flatten() {
            self.open_path(path);
        }
    }

    /// Start a full-document scan for the sidebar query on a worker thread.
    /// Any running scan is cancelled first; results stream in via
    /// [`ReaderApp::poll_search`].
    fn start_search(&mut self) {
        self.cancel_search();
        let query = self.search_query.trim().to_owned();
        if query.is_empty() {
            self.search_hits = None;
            return;
        }
        let Some(document) = self.document.clone() else {
            return;
        };
        self.search_hits = Some(Vec::new());
        self.search_job = Some(SearchJob::spawn(document, query));
    }

    /// Stop any running scan and forget its results (query changed or the
    /// document closed).
    fn cancel_search(&mut self) {
        if let Some(job) = self.search_job.take() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.search_hits = None;
    }

    /// Drain the running scan's finished batches: rows appear progressively,
    /// the first hit jumps into view once available, and a terminal backend
    /// error surfaces as the usual banner.
    fn poll_search(&mut self) {
        let Some(mut job) = self.search_job.take() else {
            return;
        };
        let (batches, done) = job.poll();
        let mut jump = None;
        let mut hits = self.search_hits.take().unwrap_or_default();
        for batch in batches {
            match batch {
                Ok(rows) => {
                    if !rows.is_empty() && !job.jumped {
                        job.jumped = true;
                        jump = Some(rows[0].page);
                    }
                    hits.extend(rows);
                }
                Err(err) => self.error = Some(err),
            }
        }
        self.search_hits = Some(hits);
        if !done {
            self.search_job = Some(job);
        }
        if let Some(page) = jump {
            self.goto_page(page, None);
        }
    }

    /// Jump to the next (`dir > 0`) or previous hit page relative to the
    /// reading position, cycling past either end.
    fn cycle_search_hit(&mut self, dir: isize) {
        let Some(hits) = self.search_hits.as_deref().filter(|hits| !hits.is_empty()) else {
            return;
        };
        let page = cycle_hit_page(hits, self.session.page, dir);
        self.goto_page(page, None);
    }

    /// Hit rects for one painted page, in trait order: lowercase the live
    /// query, reuse a cached result when it still matches, otherwise re-query
    /// the backend (errors become no rects). Empty while no search is active.
    fn page_highlight(&mut self, page: usize) -> Vec<[f32; 4]> {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() || self.search_hits.as_deref().is_none_or(|h| h.is_empty()) {
            return Vec::new();
        }
        if let Some((hit_page, hit_query, rects)) = &self.highlight
            && *hit_page == page
            && hit_query == &query
        {
            return rects.clone();
        }
        let Some(document) = self.document.as_ref() else {
            return Vec::new();
        };
        let rects = document.search_page(page, &query).unwrap_or_default();
        self.highlight = Some((page, query, rects.clone()));
        rects
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
                    // A success revokes the failure ledger entry, so a
                    // recovered page never keeps a retry schedule alive.
                    self.failed.remove(&request.key());
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
                    // Failures at a superseded scale are irrelevant — their
                    // keys can never match a current-scale request again.
                    if request.scale_q == self.current_scale_q(ctx) {
                        note_render_failure(&mut self.failed, request.key(), Instant::now(), error);
                    }
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

    /// Rebuild the layout when the available size or zoom preference changed.
    /// Page sizes are immutable per document, so they are fetched once at open
    /// and reused across relayouts. Fit-page re-derives its percent from the
    /// current viewport on every rebuild.
    fn ensure_layout(&mut self, avail_w: f32, avail_h: f32) -> bool {
        if self.fit_page && !self.page_sizes.is_empty() {
            let pct = layout::fit_page_percent(
                &self.page_sizes,
                avail_w,
                avail_h,
                layout::pages_per_row(self.flow),
            );
            self.session.zoom = ZoomMode::Percent(pct);
        }
        let key = (avail_w, avail_h, self.session.zoom, self.flow);
        if self.layout_key == Some(key) {
            return true;
        }
        if self.page_sizes.is_empty() {
            return false;
        }
        self.layout = Layout::build(&self.page_sizes, self.session.zoom, avail_w, self.flow);
        self.zoom_pct = layout::quantize_nearest(self.layout.zoom * 100.0);
        self.layout_key = Some(key);
        true
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
                // The per-page failure ledger surfaces this as a
                // click-to-retry state instead of a banner.
                note_render_failure(&mut self.failed, key, Instant::now(), err.to_string());
            }
        }
    }

    /// Recolor-at-promotion: a recolorized bitmap for `(page, scale, page
    /// colors)` is uploaded straight from the theme cache when present;
    /// otherwise a cached original is cloned, mapped once, and stored so
    /// theme switches only ever pay the first pass. Loads once per slot or
    /// reuses via `TextureHandle::set`.
    fn promote_texture(
        &mut self,
        ctx: &egui::Context,
        page: usize,
        scale_q: u16,
        fallback: Option<&PageImage>,
    ) {
        let key = RecolorKey {
            page,
            scale_q,
            theme: recolor_key(self.theme.page_bg, self.theme.page_fg),
        };
        if let Some(img) = self.recolored.get(key) {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [img.width as usize, img.height as usize],
                img.rgba,
            );
            self.load_texture(ctx, page, scale_q, image);
            return;
        }
        let (width, height, mut rgba) = match self.cache.get(CacheKey { page, scale_q }) {
            Some(img) => (img.width, img.height, img.rgba.to_vec()),
            None => match fallback {
                Some(image) => (image.width, image.height, image.rgba.clone()),
                None => return,
            },
        };
        recolor(&mut rgba, self.theme.page_bg, self.theme.page_fg);
        let image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        self.recolored.insert(key, width, height, rgba);
        self.load_texture(ctx, page, scale_q, image);
    }

    /// Push a recolorized image into the page's texture slot, loading the
    /// handle once or reusing it via `set`.
    fn load_texture(
        &mut self,
        ctx: &egui::Context,
        page: usize,
        scale_q: u16,
        image: egui::ColorImage,
    ) {
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
            prune_stale_scale(&mut self.pending, &mut self.failed, scale_q);
            self.failed_scale_q = scale_q;
        }
        let scale = self.current_scale(ctx.pixels_per_point());
        let now = Instant::now();
        let mut wanted = Vec::new();
        for page in candidates {
            let key = CacheKey { page, scale_q };
            if self
                .textures
                .get(&page)
                .is_some_and(|t| t.scale_q == scale_q)
                || !wants_render(&self.pending, &self.failed, key, now)
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
        }
        if !wanted.is_empty() {
            match queue_renders(self.pipeline.as_ref(), &wanted, &mut self.pending) {
                Submission::Refused => self.renderer_stopped(),
                Submission::Queued | Submission::NoRenderer => {}
            }
        }
    }

    /// The worker thread died; nothing will ever drain `pending`. Drop the
    /// pipeline and the failure ledger so placeholders stop promising
    /// progress and surface the failure instead of spinning silently.
    fn renderer_stopped(&mut self) {
        self.pipeline = None;
        self.pending.clear();
        self.failed.clear();
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
            ui.label(
                egui::RichText::new("Edits apply live · Ctrl+= / Ctrl+- font · Esc to close")
                    .weak(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.editor = None;
                }
            });
        });
        ui.label(
            egui::RichText::new(
                "The theme name persists across sessions; unknown names load as Dark next time.",
            )
            .weak()
            .small(),
        );

        let mut pending_save = None;
        // Collected under the `editor` borrow, merged into the registry
        // after it ends. The editor never auto-requests focus.
        let mut editor_focus = FocusGuard::default();
        {
            let Some(editor) = self.editor.as_mut() else {
                return;
            };
            ui.horizontal(|ui| {
                ui.label("Save as");
                let field = ui.add(
                    egui::TextEdit::singleline(&mut editor.saved_name)
                        .hint_text("My Theme")
                        .desired_width(140.0),
                );
                editor_focus.register(&field);
                if ui.button("Save").clicked() && !editor.saved_name.trim().is_empty() {
                    pending_save = Some(editor.saved_name.clone());
                }
            });
            if let Some(status) = &editor.saved {
                ui.label(egui::RichText::new(status).weak().small());
            }
        }
        if let Some(name) = pending_save {
            self.save_editor_theme_as(&name);
        }

        let edited = {
            let Some(editor) = self.editor.as_mut() else {
                return;
            };
            // Editor-scoped font zoom: Ctrl+= / Ctrl+- keys and Ctrl+scroll
            // (egui's zoom delta covers both pinch and ctrl+scroll).
            let mut zoom = 1.0_f32;
            ui.ctx().input(|input| {
                if !input.modifiers.ctrl {
                    return;
                }
                if input.key_pressed(Key::Equals) || input.key_pressed(Key::Plus) {
                    zoom = EDITOR_FONT_STEP;
                } else if input.key_pressed(Key::Minus) {
                    zoom = 1.0 / EDITOR_FONT_STEP;
                } else {
                    zoom = input.zoom_delta();
                }
            });
            if zoom != 1.0 {
                editor.font_size = (editor.font_size * zoom)
                    .clamp(*EDITOR_FONT_RANGE.start(), *EDITOR_FONT_RANGE.end());
            }
            if let Some(err) = &editor.error {
                ui.colored_label(ERROR_RED, err);
            }
            let mut buffer = std::mem::take(&mut editor.buffer);
            let mut layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                let job = yaml_job(text, &self.theme, editor.font_size);
                ui.fonts(|f| f.layout_job(job))
            };
            let edit_box = egui::TextEdit::multiline(&mut buffer)
                .font(egui::TextStyle::Monospace)
                .layouter(&mut layouter);
            let response = ui.add_sized([ui.available_width(), ui.available_height()], edit_box);
            editor_focus.register(&response);
            if response.changed() {
                editor.edit(buffer)
            } else {
                editor.buffer = buffer;
                None
            }
        };
        self.focus.editables.append(&mut editor_focus.editables);
        if let Some(theme) = edited {
            self.apply_edited_theme(theme);
        }
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        // A resize-driven relayout rescales the content (fit flows re-derive
        // their percent from the new width); remember what the viewport
        // center is looking at so the rebuild keeps it in place.
        let resize_anchor = center_anchor(&self.layout, self.canvas_center_y);
        let old_zoom = self.zoom_pct;
        if !self.ensure_layout(ui.available_width(), ui.available_height()) {
            return;
        }
        if self.scroll_anchor.is_none()
            && self.pending_scroll.is_none()
            && self.zoom_pct != old_zoom
        {
            self.scroll_anchor = resize_anchor;
        }
        // Backdrop over the entire clip rect: at high zoom the content block
        // is narrower than the viewport and unpainted regions showed through.
        // Pulled darker than the sidebar so pages read as sheets on it.
        ui.painter()
            .rect_filled(ui.clip_rect(), 0.0, canvas_surface(&self.theme));
        let ctx = ui.ctx().clone();
        if !self.primed {
            self.primed = true;
            // Reopening restores the session-default UI scale declared by
            // [`ReaderApp::open_path`].
            ctx.set_pixels_per_point(self.base_ppp * self.ui_scale);
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
        // A zoom or flow change pending its relayout restores the recorded
        // anchor point (page + depth-fraction) to the viewport center.
        if let Some(anchor) = self.scroll_anchor.take()
            && let Some(y) = anchored_offset(&self.layout, anchor, ui.clip_rect().height())
        {
            self.pending_scroll = Some(y);
        }
        let jump = self.pending_scroll.take();

        // Reading scrollbar: a floating, always-visible fg-colored thumb —
        // wider than the old 4 pt hairline, no layout side-effects, and the
        // sidebar's own scroll style is untouched.
        let scroll = &mut ui.style_mut().spacing.scroll;
        scroll.bar_width = 8.0;
        scroll.floating_width = 7.0;
        scroll.dormant_handle_opacity = 0.7;
        scroll.active_handle_opacity = 0.85;
        ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
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

        let mut retry_click = None;
        let scale_q = self.current_scale_q(&ctx);
        let output = area.show(ui, |ui| {
            let content_w = ui.available_width();
            let (content_rect, _) = ui.allocate_exact_size(
                egui::vec2(content_w, self.layout.total_height),
                egui::Sense::hover(),
            );
            // Pages wider than the viewport (percent zoom) bleed past the
            // content block's edges symmetrically instead of hard-clipping
            // at the block boundary, so both edges crop like SumatraPDF.
            let paint_rect =
                egui::Rect::from_x_y_ranges(ui.clip_rect().x_range(), content_rect.y_range());
            let painter = ui.painter_at(paint_rect);
            let clip = ui.clip_rect();
            let top = clip.top() - content_rect.top();
            view.visible = self.layout.visible_range(top, clip.height());
            view.center_y = top + clip.height() * 0.5;
            view.clip_h = clip.height();

            let panel = canvas_surface(&self.theme);
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
                        let key = CacheKey { page, scale_q };
                        match self.failed.get(&key) {
                            Some(
                                state @ FailState {
                                    next_retry: None, ..
                                },
                            ) => {
                                let retry = ui
                                    .interact(
                                        screen,
                                        egui::Id::new(("page_retry", page)),
                                        egui::Sense::click(),
                                    )
                                    .on_hover_text(&state.detail);
                                let tint = if retry.hovered() {
                                    ERROR_RED
                                } else {
                                    ERROR_RED.gamma_multiply(0.8)
                                };
                                self.icons.paint_at(
                                    ui,
                                    egui::Rect::from_center_size(
                                        screen.center() - egui::vec2(0.0, 12.0),
                                        egui::vec2(22.0, 22.0),
                                    ),
                                    Icon::X,
                                    tint,
                                );
                                painter.text(
                                    screen.center() + egui::vec2(0.0, 14.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Render failed — click to retry",
                                    hint_font.clone(),
                                    fg.gamma_multiply(0.7),
                                );
                                if retry.clicked() {
                                    retry_click = Some(key);
                                }
                            }
                            _ => {
                                painter.text(
                                    screen.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "rendering…",
                                    hint_font.clone(),
                                    fg.gamma_multiply(0.6),
                                );
                            }
                        }
                    }
                }
                let hits = self.page_highlight(page);
                if !hits.is_empty()
                    && let Some(&(page_w, page_h)) = self.page_sizes.get(page)
                {
                    // Hit rects are fractional points over page_size points;
                    // scale both axes by the on-screen page rect.
                    for &[x0, y0, x1, y1] in &hits {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                screen.min + egui::vec2(x0 * rect.w / page_w, y0 * rect.h / page_h),
                                screen.min + egui::vec2(x1 * rect.w / page_w, y1 * rect.h / page_h),
                            ),
                            2.0,
                            color_of(self.theme.accent).gamma_multiply(0.35),
                        );
                    }
                }
                painter.rect_stroke(screen, PAGE_ROUNDING, border);
            }
        });

        if let Some(key) = retry_click {
            // Clear the terminal state; the next request_pages requeues it.
            self.failed.remove(&key);
        }

        if self.page_count() > 0 {
            if let Some(page) = self.layout.page_at(view.center_y) {
                if should_toast(self.last_toasted_page, page) {
                    self.toast = Some((page, Instant::now()));
                    self.last_toasted_page = Some(page);
                }
                self.session.page = page;
            }
            self.canvas_center_y = view.center_y;
            self.request_pages(&ctx, view.visible);
            self.prune_textures();
            let span = (self.layout.total_height - view.clip_h).max(1.0);
            self.session.scroll_frac =
                (f64::from(output.state.offset.y) / f64::from(span)).clamp(0.0, 1.0);
            match self.debug_relayout.take() {
                Some(DebugRelayout::Dual) => self.set_flow(Flow::Dual),
                Some(DebugRelayout::Single) => self.set_flow(Flow::Single),
                Some(DebugRelayout::Percent) => self.set_zoom_percent(200.0),
                None => {}
            }
        }
    }

    // --- chrome ----------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        chrome_style(ui);
        let fg = color_of(self.theme.ui_fg);
        let accent = color_of(self.theme.accent);
        ui.horizontal(|ui| {
            ui.set_height(ui.available_height());
            self.icon_menu(ui, Icon::Menu, "Menu", Self::app_menu);
            let brand = ui.label(
                egui::RichText::new("Candi")
                    .strong()
                    .size(19.0)
                    .color(accent),
            );
            // Decorations are off, so the brand zone doubles as the drag
            // handle; double-click toggles maximize like a native title bar.
            let drag = ui.interact(brand.rect, egui::Id::new("title_drag"), egui::Sense::drag());
            if drag.dragged() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self
                    .icons
                    .button(ui, Icon::Focus, 26.0, fg)
                    .on_hover_text("Focus mode (F11)")
                    .clicked()
                {
                    self.focus_mode = !self.focus_mode;
                }
                if self
                    .icons
                    .button(ui, Icon::Info, 26.0, fg)
                    .on_hover_text("Document information")
                    .clicked()
                {
                    self.info_open = true;
                }
                let search_color = if self.sidebar_open && self.section == SidebarSection::Search {
                    accent
                } else {
                    fg
                };
                if self
                    .icons
                    .button(ui, Icon::Search, 26.0, search_color)
                    .on_hover_text("Search (Ctrl+F)")
                    .clicked()
                {
                    self.sidebar_open = true;
                    self.section = SidebarSection::Search;
                    self.focus_search = true;
                }
            });
        });
        // The pill is page chrome — a floating "n / N" makes no sense on
        // the welcome screen, so it only exists with a document.
        if self.page_count() > 0 {
            self.nav_cluster(ui, fg);
        }
    }

    /// Icon button that opens a popup menu — the font-free replacement for
    /// `menu_button`, which only accepts text.
    fn icon_menu(
        &mut self,
        ui: &mut egui::Ui,
        icon: Icon,
        tip: &str,
        contents: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let resp = self
            .icons
            .button(ui, icon, 26.0, color_of(self.theme.ui_fg));
        let resp = resp.on_hover_text(tip);
        let id = resp.id.with("popup");
        if resp.clicked() {
            ui.memory_mut(|mem| mem.toggle_popup(id));
        }
        if ui.memory(|mem| mem.is_popup_open(id)) {
            egui::popup_below_widget(
                ui,
                id,
                &resp,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_min_width(240.0);
                    contents(self, ui);
                },
            );
        }
    }

    /// Secondary actions menu (⋮): everything else, mockup §6.
    fn app_menu(&mut self, ui: &mut egui::Ui) {
        let fg = color_of(self.theme.ui_fg);
        let focus_label = if self.focus_mode {
            "Exit Focus Mode"
        } else {
            "Focus Mode"
        };
        if menu_item(
            ui,
            &mut self.icons,
            Some(Icon::Page),
            "Open File…",
            Some("Ctrl+O"),
            fg,
        )
        .clicked()
        {
            ui.memory_mut(|mem| mem.close_popup());
            self.open_dialog();
        }
        if menu_item(
            ui,
            &mut self.icons,
            Some(Icon::Save),
            "Save State",
            Some("Ctrl+S"),
            fg,
        )
        .clicked()
        {
            ui.memory_mut(|mem| mem.close_popup());
            self.save_state();
        }
        if menu_item(
            ui,
            &mut self.icons,
            Some(Icon::Info),
            "Document Information…",
            None,
            fg,
        )
        .clicked()
        {
            ui.memory_mut(|mem| mem.close_popup());
            self.info_open = true;
        }
        if menu_item(
            ui,
            &mut self.icons,
            Some(Icon::Gear),
            "Edit Config (YAML)…",
            Some("Ctrl+E"),
            fg,
        )
        .clicked()
        {
            ui.memory_mut(|mem| mem.close_popup());
            self.open_theme_editor();
        }
        if menu_item(
            ui,
            &mut self.icons,
            Some(Icon::Focus),
            focus_label,
            Some("F11"),
            fg,
        )
        .clicked()
        {
            ui.memory_mut(|mem| mem.close_popup());
            self.focus_mode = !self.focus_mode;
        }
        if menu_item(
            ui,
            &mut self.icons,
            Some(Icon::List),
            "Keyboard Shortcuts",
            None,
            fg,
        )
        .clicked()
        {
            ui.memory_mut(|mem| mem.close_popup());
            self.shortcuts_open = true;
        }
        if menu_item(ui, &mut self.icons, None, "About Candi", None, fg).clicked() {
            ui.memory_mut(|mem| mem.close_popup());
            self.about_open = true;
        }
    }

    /// Bordered `‹ n / N ›` pill (mockup §4), floating dead-center of the
    /// WINDOW: an area anchored to the screen's top-center, since egui's
    /// horizontal main_align only positions content inside widgets, not the
    /// widget stream. The count opens the inline page-jump input.
    fn nav_cluster(&mut self, ui: &mut egui::Ui, fg: egui::Color32) {
        let count = self.page_count();
        let current = self.session.page;
        let mut done = false;
        egui::Area::new(egui::Id::new("page_pill"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 10.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::default()
                    .stroke(egui::Stroke::new(1.0_f32, fg.gamma_multiply(0.25)))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add_enabled_ui(count > 0 && current > 0, |ui| {
                                if self
                                    .icons
                                    .button(ui, Icon::ChevronLeft, 22.0, fg)
                                    .on_hover_text("Previous page (Left / PgUp / -)")
                                    .clicked()
                                {
                                    self.goto_page(current - 1, None);
                                }
                            });
                            match self.page_jump.as_mut() {
                                Some(buffer) => {
                                    let outcome = jump_input(
                                        ui,
                                        buffer,
                                        count,
                                        &mut self.page_jump_focus,
                                        &mut self.jump_invalid,
                                        &mut self.focus,
                                        &mut self.escape_consumed,
                                    );
                                    match outcome {
                                        JumpOutcome::Commit(page) => {
                                            self.goto_page(page, None);
                                            done = true;
                                        }
                                        JumpOutcome::Cancel => done = true,
                                        JumpOutcome::Idle => {}
                                    }
                                }
                                None if count > 0 => {
                                    let counter = format!("{} / {}", current + 1, count);
                                    let counter = egui::Label::new(
                                        egui::RichText::new(counter)
                                            .font(egui::FontId::proportional(13.0)),
                                    )
                                    .sense(egui::Sense::click());
                                    if ui
                                        .add(counter)
                                        .on_hover_text("Jump to page (Ctrl+G)")
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                        && count > 0
                                    {
                                        self.page_jump = Some((current + 1).to_string());
                                        self.page_jump_focus = true;
                                        self.jump_invalid = false;
                                    }
                                }
                                None => {}
                            }
                            ui.add_enabled_ui(count > 0 && current + 1 < count, |ui| {
                                if self
                                    .icons
                                    .button(ui, Icon::ChevronRight, 22.0, fg)
                                    .on_hover_text("Next page (Right / PgDn / +)")
                                    .clicked()
                                {
                                    self.goto_page(current + 1, None);
                                }
                            });
                        });
                    });
            });
        if done {
            self.page_jump = None;
        }
    }

    /// The section panel beside the rail: header + scrollable body. Width
    /// comes from the resizable SidePanel (230–400 px, user-dragged).
    fn section_panel(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .fill(color_of(tinted_chrome(&self.theme).panel_bg))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let (label, salt, body): (&str, &str, fn(&mut ReaderApp, &mut egui::Ui)) =
                        match self.section {
                            SidebarSection::Contents => {
                                ("CONTENTS", "sidebar_contents", ReaderApp::show_contents)
                            }
                            SidebarSection::Bookmarks => {
                                ("BOOKMARKS", "sidebar_bookmarks", ReaderApp::show_bookmarks)
                            }
                            SidebarSection::Search => {
                                ("SEARCH", "sidebar_search", ReaderApp::show_search)
                            }
                            SidebarSection::Appearance => (
                                "APPEARANCE",
                                "sidebar_appearance",
                                ReaderApp::show_appearance,
                            ),
                        };
                    ui.label(egui::RichText::new(label).weak().small());
                    ui.add_space(4.0);
                    ui.style_mut().spacing.scroll.bar_width = 4.0;
                    ui.visuals_mut().extreme_bg_color = egui::Color32::TRANSPARENT;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt(salt)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| body(self, ui));
                });
            });
    }

    /// Appearance panel: theme picker, accent swatches, and UI text scale.
    /// Accent/size tweaks are session-local; the YAML editor is the way to
    /// keep them.
    fn show_appearance(&mut self, ui: &mut egui::Ui) {
        let fg = color_of(self.theme.ui_fg);
        ui.label(egui::RichText::new("Theme").weak().small());
        self.theme_picker(ui, fg);
        ui.add_space(12.0);

        ui.label(egui::RichText::new("Accent color").weak().small());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for &rgb in &ACCENT_SWATCHES {
                let swatch = Color::from([rgb[0], rgb[1], rgb[2], 0xFF]);
                let selected = [
                    self.theme.accent.r(),
                    self.theme.accent.g(),
                    self.theme.accent.b(),
                ] == rgb;
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(SWATCH_SIZE, SWATCH_SIZE),
                    egui::Sense::click(),
                );
                if resp.clicked() {
                    self.set_accent(ui.ctx(), rgb);
                }
                let color = color_of(swatch);
                let center = rect.center();
                ui.painter().circle_filled(center, SWATCH_SIZE / 2.0, color);
                if selected {
                    ui.painter().circle_stroke(
                        center,
                        SWATCH_SIZE / 2.0 - 1.0,
                        egui::Stroke::new(2.0_f32, color.gamma_multiply(0.5)),
                    );
                }
            }
        });
        ui.add_space(12.0);

        ui.label(egui::RichText::new("Text size").weak().small());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("A").weak());
            let mut scale = self.ui_scale;
            if ui
                .add(
                    egui::Slider::new(&mut scale, UI_SCALE_RANGE)
                        .step_by(0.05)
                        .show_value(false),
                )
                .changed()
            {
                self.ui_scale = scale;
                ui.ctx().set_pixels_per_point(self.base_ppp * scale);
            }
            ui.label(egui::RichText::new("A").font(egui::FontId::proportional(17.0)));
        });
        ui.add_space(12.0);

        if ui.selectable_label(false, "Edit Config (YAML)…").clicked() {
            self.open_theme_editor();
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Accent and text size reset when the document reopens — save them in the YAML to keep them.",
            )
            .weak(),
        );
    }

    /// The ~52 px rail: sections top-down, appearance pinned bottom.
    fn rail(&mut self, ui: &mut egui::Ui) {
        let accent = color_of(self.theme.accent);
        let fg = color_of(self.theme.ui_fg);
        ui.vertical(|ui| {
            ui.set_width(52.0);
            chrome_style(ui);
            let sections = [
                (SidebarSection::Contents, Icon::List, "Contents"),
                (SidebarSection::Bookmarks, Icon::Flag, "Bookmarks"),
                (SidebarSection::Search, Icon::Search, "Search"),
            ];
            let (rail_rect, _) = ui.allocate_exact_size(
                egui::vec2(52.0, ui.available_height()),
                egui::Sense::hover(),
            );
            for (index, (section, icon, name)) in sections.iter().enumerate() {
                let active = self.sidebar_open && self.section == *section;
                let color = if active { accent } else { fg };
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rail_rect.left() + 7.0,
                        rail_rect.top() + 8.0 + 46.0 * index as f32,
                    ),
                    egui::vec2(38.0, 38.0),
                );
                let button =
                    egui::Button::image(self.icons.image(ui, *icon, 26.0, color)).rounding(6.0);
                let button = ui
                    .put(rect, button)
                    .on_hover_text(*name)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if active {
                    let bar_h = rect.height() * 0.6;
                    ui.painter().rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(rail_rect.left() + 1.5, rect.center().y),
                            egui::vec2(3.0, bar_h),
                        ),
                        1.5,
                        accent,
                    );
                }
                if button.clicked() {
                    if active {
                        self.sidebar_open = false;
                    } else {
                        self.sidebar_open = true;
                        self.section = *section;
                        if *section == SidebarSection::Search {
                            self.focus_search = true;
                        }
                    }
                }
            }
            let toggle_rect = egui::Rect::from_min_size(
                egui::pos2(rail_rect.left() + 7.0, rail_rect.top() + 8.0 + 46.0 * 3.0),
                egui::vec2(38.0, 38.0),
            );
            let toggle_icon = if self.sidebar_open {
                Icon::PanelClose
            } else {
                Icon::PanelOpen
            };
            let toggle =
                egui::Button::image(self.icons.image(ui, toggle_icon, 26.0, fg)).rounding(6.0);
            if ui
                .put(toggle_rect, toggle)
                .on_hover_text("Toggle sidebar (Ctrl+B)")
                .clicked()
            {
                self.sidebar_open = !self.sidebar_open;
            }
            let gear_rect = egui::Rect::from_min_size(
                egui::pos2(rail_rect.left() + 7.0, rail_rect.bottom() - 46.0),
                egui::vec2(38.0, 38.0),
            );
            let gear =
                egui::Button::image(self.icons.image(ui, Icon::Gear, 26.0, fg)).rounding(6.0);
            if ui
                .put(gear_rect, gear)
                .on_hover_text("Appearance")
                .clicked()
            {
                self.sidebar_open = true;
                self.section = SidebarSection::Appearance;
            }
        });
    }

    fn show_contents(&mut self, ui: &mut egui::Ui) {
        if self.page_count() == 0 {
            ui.label(egui::RichText::new(NO_DOC).weak());
            return;
        }
        if self.toc_rows.is_empty() {
            ui.label(egui::RichText::new("No table of contents").weak());
            return;
        }
        // The last-clicked row keeps the accent until the reader scrolls to
        // a different page.
        if let Some((_, page)) = self.toc_follow
            && page != self.session.page
        {
            self.toc_follow = None;
        }
        // Reading height within the current page, in points from its top —
        // the same space as the outline destinations.
        let pos = self.layout.rects.get(self.session.page).map(|rect| {
            let within = ((self.canvas_center_y - rect.y) / self.layout.zoom).max(0.0);
            (self.session.page, within)
        });
        let computed = active_toc_row(&self.toc_rows, pos);
        let active = self.toc_follow.map_or(computed, |(row, _)| Some(row));
        let accent = color_of(self.theme.accent);
        let mut jump = None;
        for (idx, row) in self.toc_rows.iter().enumerate() {
            if toc_row_ui(ui, row, active == Some(idx), accent).clicked() {
                jump = Some((row.page, row.dest_top));
                self.toc_follow = Some((idx, row.page));
            }
        }
        if let Some((page, dest)) = jump {
            self.goto_page(page, dest);
        }
    }

    fn show_bookmarks(&mut self, ui: &mut egui::Ui) {
        if self.page_count() == 0 {
            ui.label(egui::RichText::new(NO_DOC).weak());
            return;
        }
        let fg = color_of(self.theme.ui_fg);
        if ui.button("Add bookmark").clicked() {
            self.note_engagement();
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
        let mut begin_rename = None;
        let mut commit_rename: Option<(usize, String)> = None;
        let mut cancel_rename = false;
        for bookmark in &self.session.bookmarks {
            if self.renaming == Some(bookmark.page) {
                ui.horizontal(|ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_buffer)
                            .desired_width(ui.available_width() - 30.0),
                    );
                    self.focus.register(&field);
                    if self.rename_focus {
                        self.focus.request(&field);
                        self.rename_focus = false;
                    }
                    if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        commit_rename =
                            Some((bookmark.page, std::mem::take(&mut self.rename_buffer)));
                    } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                        cancel_rename = true;
                        self.rename_buffer.clear();
                        self.escape_consumed = true;
                    }
                    if self.icons.button(ui, Icon::X, 18.0, fg).clicked() {
                        cancel_rename = true;
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    let row = if let Some(title) = bookmark.title.as_deref() {
                        format!("{title} — {}", date_only(&bookmark.created_at))
                    } else {
                        format!(
                            "Page {} — {}",
                            bookmark.page + 1,
                            date_only(&bookmark.created_at)
                        )
                    };
                    let reserve =
                        2.0 * icon_button_width(ui, 18.0) + 2.0 * ui.spacing().item_spacing.x;
                    if click_row(
                        ui,
                        &row,
                        ui.visuals().text_color(),
                        ui.available_width() - reserve,
                    )
                    .clicked()
                    {
                        jump = Some(bookmark.page);
                    }
                    if self
                        .icons
                        .button(ui, Icon::Pen, 18.0, fg)
                        .on_hover_text("Rename")
                        .clicked()
                    {
                        begin_rename = Some(bookmark.page);
                    }
                    if self.icons.button(ui, Icon::X, 18.0, fg).clicked() {
                        remove = Some(bookmark.page);
                    }
                });
            }
        }
        if let Some((page, title)) = commit_rename {
            self.session.rename_bookmark(page, title);
            self.renaming = None;
        } else if cancel_rename {
            self.renaming = None;
            self.rename_buffer.clear();
        }
        if let Some(page) = begin_rename {
            self.renaming = Some(page);
            self.rename_buffer = self
                .session
                .bookmarks
                .iter()
                .find(|b| b.page == page)
                .and_then(|b| b.title.clone())
                .unwrap_or_default();
            self.rename_focus = true;
        }
        if let Some(page) = jump {
            self.goto_page(page, None);
        }
        if let Some(page) = remove {
            self.session.remove_bookmark(page);
        }
    }

    fn show_search(&mut self, ui: &mut egui::Ui) {
        if self.page_count() == 0 {
            ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Find in document")
                    .desired_width(ui.available_width()),
            );
            ui.label(egui::RichText::new(NO_DOC).weak());
            return;
        }
        let fg = color_of(self.theme.ui_fg);
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Find in document")
                .desired_width(ui.available_width()),
        );
        self.focus.register(&field);
        if self.focus_search {
            self.focus.request(&field);
            self.focus_search = false;
        }
        if field.changed() {
            self.cancel_search();
        }
        if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            self.start_search();
        }

        let total = self.search_hits.as_deref().map_or(0, <[SearchHit]>::len);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{total} matches")).weak());
            if self.search_job.is_some() {
                ui.label(egui::RichText::new("searching…").weak());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(total > 0, |ui| {
                    if self
                        .icons
                        .button(ui, Icon::ChevronRight, 18.0, fg)
                        .on_hover_text("Next match")
                        .clicked()
                    {
                        self.cycle_search_hit(1);
                    }
                    if self
                        .icons
                        .button(ui, Icon::ChevronLeft, 18.0, fg)
                        .on_hover_text("Previous match")
                        .clicked()
                    {
                        self.cycle_search_hit(-1);
                    }
                });
            });
        });

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
                    let label = format!("p. {} — {}", hit.page + 1, hit.snippet);
                    if click_row(ui, &label, ui.visuals().text_color(), ui.available_width())
                        .clicked()
                    {
                        jump = Some(hit.page);
                    }
                }
            }
        };
        if let Some(page) = jump {
            self.goto_page(page, None);
        }
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        chrome_style(ui);
        let fg = color_of(self.theme.ui_fg);
        let accent = color_of(self.theme.accent);
        ui.columns(3, |columns| {
            columns[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if self
                    .icons
                    .button(ui, theme_icon(&self.theme), 24.0, accent)
                    .on_hover_text("Cycle themes (T)")
                    .clicked()
                {
                    self.cycle_theme();
                }
                self.theme_picker(ui, fg);
            });
            columns[1].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                self.zoom_slider(ui, fg);
            });
            columns[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.view_modes(ui, fg);
            });
        });
    }

    /// Bordered theme picker: current name + chevron; popup lists the
    /// built-ins, custom themes (with delete affordance), and the YAML
    /// editor. No font glyphs anywhere.
    fn theme_picker(&mut self, ui: &mut egui::Ui, fg: egui::Color32) {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(92.0, 26.0), egui::Sense::click());
        let fill = if resp.hovered() {
            ui.visuals().widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 6.0, fill);
        // Truncate long names so the chevron keeps a clear gap at every
        // name length (both picker placements share this function).
        let name_font = egui::FontId::proportional(13.0);
        let name = truncate_to_width(
            &|s: &str| {
                ui.painter()
                    .layout_no_wrap(s.to_owned(), name_font.clone(), fg)
                    .rect
                    .width()
            },
            &self.theme.name,
            rect.width() - 10.0 - 25.0,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &name,
            name_font,
            fg,
        );
        self.icons.paint_at(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(rect.right() - 14.0, rect.center().y),
                egui::vec2(14.0, 14.0),
            ),
            Icon::ChevronDown,
            fg.gamma_multiply(0.7),
        );
        let id = resp.id.with("theme_popup");
        if resp.clicked() {
            ui.memory_mut(|mem| mem.toggle_popup(id));
        }
        if self.debug_picker {
            self.debug_picker = false;
            ui.memory_mut(|mem| mem.open_popup(id));
        }
        if ui.memory(|mem| mem.is_popup_open(id)) {
            egui::popup_below_widget(
                ui,
                id,
                &resp,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_min_width(170.0);
                    for name in BUILTIN_NAMES {
                        if ui
                            .selectable_label(self.theme.name == *name, name)
                            .clicked()
                        {
                            self.set_theme(name);
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    }
                    // Names cloned out so deleting a row can mutate the
                    // registry mid-popup.
                    let custom_names: Vec<String> = self
                        .custom_themes
                        .iter()
                        .map(|(theme, _)| theme.name.clone())
                        .collect();
                    ui.separator();
                    ui.label(egui::RichText::new("My Themes").weak().small());
                    if custom_names.is_empty() {
                        ui.label(
                            egui::RichText::new("Create one via Save As in the editor")
                                .weak()
                                .small(),
                        );
                    }
                    for name in &custom_names {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.theme.name == *name, name)
                                .clicked()
                            {
                                self.set_theme(name);
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if self
                                        .icons
                                        .button(ui, Icon::X, 14.0, fg)
                                        .on_hover_text("Delete this custom theme")
                                        .clicked()
                                    {
                                        self.delete_custom_theme(name);
                                    }
                                },
                            );
                        });
                    }
                    if let Some(status) = &self.theme_status {
                        ui.label(egui::RichText::new(status).weak().small());
                    }
                    ui.separator();
                    if ui.selectable_label(false, "Edit Config (YAML)…").clicked() {
                        ui.memory_mut(|mem| mem.close_popup());
                        self.open_theme_editor();
                    }
                },
            );
        }
    }

    /// 100% − slider + (states 1–6 bottom bar).
    fn zoom_slider(&mut self, ui: &mut egui::Ui, fg: egui::Color32) {
        let can_zoom = self.page_count() > 0;
        let pct_font = egui::FontId::proportional(13.0);
        let label = format!("{}%", self.zoom_pct);
        // Measured cluster width: two 22 pt icon buttons (9 pt padding each
        // side, per chrome_style), the % label, and the spacing between the
        // four widgets.
        let cluster_w = 80.0
            + 3.0 * ui.spacing().item_spacing.x
            + ui.painter()
                .layout_no_wrap(label.clone(), pct_font.clone(), egui::Color32::WHITE)
                .rect
                .width();
        let (lead, slider_w) = zoom_centering(ui.available_width(), cluster_w);
        ui.add_space(lead);
        ui.add_enabled_ui(can_zoom, |ui| {
            if self.icons.button(ui, Icon::Minus, 22.0, fg).clicked() {
                self.zoom_step(-5);
            }
        });
        ui.label(egui::RichText::new(&label).font(pct_font))
            .on_hover_text("Zoom");
        ui.add_enabled_ui(can_zoom, |ui| {
            if self.icons.button(ui, Icon::Plus, 22.0, fg).clicked() {
                self.zoom_step(5);
            }
        });
        let mut pct = i32::from(self.zoom_pct);
        let slider = ui.scope(|ui| {
            // The width zoom_centering allotted; shrinking it first keeps the
            // cluster inside its bottom-bar third at the 640 px minimum
            // window width.
            ui.spacing_mut().slider_width = slider_w;
            ui.spacing_mut().interact_size.y = 12.0;
            ui.add_enabled(
                can_zoom,
                egui::Slider::new(
                    &mut pct,
                    i32::from(layout::MIN_ZOOM_PERCENT)..=i32::from(SLIDER_MAX_PERCENT),
                )
                .step_by(5.0)
                .show_value(false),
            )
        });
        if slider.inner.changed() {
            self.set_zoom_percent(pct as f32);
        }
        self.zoom_slider_rect = Some(slider.inner.rect);
    }

    /// View-mode toggles — continuous, single page, dual spreads, fit page
    /// (visual order left to right; emitted right-to-left). A flow pick
    /// refits to its widest row via [`ReaderApp::set_flow`].
    fn view_modes(&mut self, ui: &mut egui::Ui, fg: egui::Color32) {
        let can = self.page_count() > 0;
        let accent = color_of(self.theme.accent);
        let on_flow = !self.fit_page;
        let continuous = on_flow && self.flow == Flow::Continuous;
        let single = on_flow && self.flow == Flow::Single;
        let dual = on_flow && self.flow == Flow::Dual;
        let fitp = self.fit_page;
        ui.add_enabled_ui(can, |ui| {
            if self
                .icons
                .button(ui, Icon::Expand, 24.0, if fitp { accent } else { fg })
                .on_hover_text("Fit page")
                .clicked()
            {
                self.fit_page = true;
            }
            if self
                .icons
                .button(ui, Icon::Columns2, 24.0, if dual { accent } else { fg })
                .on_hover_text("Dual-page spreads")
                .clicked()
            {
                self.set_flow(Flow::Dual);
            }
            if self
                .icons
                .button(ui, Icon::Page, 24.0, if single { accent } else { fg })
                .on_hover_text("Single page")
                .clicked()
            {
                self.set_flow(Flow::Single);
            }
            if self
                .icons
                .button(ui, Icon::Book, 24.0, if continuous { accent } else { fg })
                .on_hover_text("Continuous scroll")
                .clicked()
            {
                self.set_flow(Flow::Continuous);
            }
        });
    }

    /// §40 empty state: brand, one call to action, drag-drop hint, and the
    /// recent-documents list when the config has any — the whole block
    /// centered on both axes of the canvas area.
    fn empty_state(&mut self, ui: &mut egui::Ui) {
        let recents: Vec<(PathBuf, String)> = self
            .config
            .recents
            .iter()
            .filter_map(|recent| {
                let name = recent.path.file_name()?.to_str()?;
                Some((recent.path.clone(), name.to_owned()))
            })
            .collect();
        let mut open = None;
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let accent = color_of(self.theme.accent);
            ui.add_space(centered_top_offset(
                ui.available_height(),
                welcome_block_height(ui, recents.len(), !self.config.onboarding_done),
            ));
            let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
            self.icons
                .paint_at(ui, rect, Icon::Page, accent.gamma_multiply(0.9));
            ui.add_space(14.0);
            ui.label(egui::RichText::new("No file open").strong().size(18.0));
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Open a PDF to get started with Candi.").weak());
            ui.add_space(18.0);
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("   Open File   ").strong())
                        .fill(accent)
                        .rounding(8.0)
                        .min_size(egui::vec2(150.0, 36.0)),
                )
                .clicked()
            {
                self.open_dialog();
            }
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("or drag and drop a PDF here")
                    .weak()
                    .small(),
            );
            if !self.config.onboarding_done {
                ui.add_space(26.0);
                ui.label(egui::RichText::new("Quick start").weak().small());
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Ctrl+O open · Ctrl+K shortcuts · Ctrl+E themes")
                        .weak()
                        .small(),
                );
                ui.label(
                    egui::RichText::new("Reading position saves automatically.")
                        .weak()
                        .small(),
                );
                ui.add_space(6.0);
                if ui.small_button("Got it").clicked() {
                    self.dismiss_onboarding();
                }
            }
            if !recents.is_empty() {
                ui.add_space(26.0);
                ui.label(egui::RichText::new("Recent").weak().small());
                ui.add_space(2.0);
                for (path, name) in &recents {
                    let weak = ui.visuals().weak_text_color();
                    let font = egui::TextStyle::Body.resolve(ui.style());
                    let strip = ui
                        .painter()
                        .layout_no_wrap(name.clone(), font, weak)
                        .rect
                        .width()
                        + 12.0;
                    if click_row(ui, name, weak, strip)
                        .on_hover_text(path.display().to_string())
                        .clicked()
                    {
                        open = Some(path.clone());
                    }
                }
            }
        });
        if let Some(path) = open {
            self.open_path(path);
        }
    }

    fn open_error_view(&mut self, ui: &mut egui::Ui) {
        let raw = self.error.clone().unwrap_or_default();
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let h = ui.available_height();
            ui.add_space(h * 0.30);
            ui.label(
                egui::RichText::new("Unable to open this PDF.")
                    .strong()
                    .size(18.0),
            );
            ui.label(egui::RichText::new(humanize_reason(&raw)).weak());
            ui.add_space(8.0);
            egui::CollapsingHeader::new("Details")
                .id_salt("open_error_details")
                .show_unindented(ui, |ui| {
                    ui.monospace(&raw);
                });
            ui.add_space(4.0);
            if ui.button("Open another file").clicked() {
                self.open_dialog();
            }
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

    fn info_window(&mut self, ctx: &egui::Context) {
        let mut open = self.info_open;
        egui::Window::new("Document Information")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                if self.page_count() == 0 {
                    ui.label(egui::RichText::new(NO_DOC).weak());
                    return;
                }
                egui::Grid::new("doc_info")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Title");
                        ui.label(&self.filename);
                        ui.end_row();
                        ui.label("Pages");
                        ui.label(self.page_count().to_string());
                        ui.end_row();
                        ui.label("Backend");
                        ui.label(format!("{:?}", self.backend).to_lowercase());
                        ui.end_row();
                    });
            });
        self.info_open = open;
    }

    fn shortcuts_window(&mut self, ctx: &egui::Context) {
        let rows = self.keybinds.rows();
        let hint = match &self.keybinds.path {
            Some(path) => format!("Edit {} to change these.", path.display()),
            None => "Set XDG_CONFIG_HOME or HOME to enable keybinds editing.".to_owned(),
        };
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut self.shortcuts_open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                let filter = ui.add(
                    egui::TextEdit::singleline(&mut self.shortcut_filter)
                        .hint_text("Filter shortcuts")
                        .desired_width(ui.available_width()),
                );
                self.focus.register(&filter);
                let needle = self.shortcut_filter.trim().to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(ctx.screen_rect().height() * 0.6)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("shortcuts_grid")
                            .num_columns(2)
                            .spacing([16.0, 6.0])
                            .show(ui, |ui| {
                                for (action, keys) in rows {
                                    if !needle.is_empty()
                                        && !format!("{keys} {action}")
                                            .to_lowercase()
                                            .contains(&needle)
                                    {
                                        continue;
                                    }
                                    ui.strong(keys);
                                    ui.label(action);
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(2.0);
                ui.label(egui::RichText::new(hint).weak().small());
                ui.label(
                    egui::RichText::new(
                        "Pinch-zoom not supported (Wayland/winit limitation) — use Ctrl+scroll to zoom.",
                    )
                    .weak()
                    .small(),
                );
            });
    }
    /// Floating "Page N of M" toast anchored above the bottom bar, centered
    /// over the zoom slider: it refreshes on every viewport-center page
    /// change, holds while scrolling, and fades out shortly after scrolling
    /// settles.
    fn show_page_toast(&mut self, ctx: &egui::Context) {
        let Some((page, shown)) = self.toast else {
            return;
        };
        let opacity = toast_opacity(shown, Instant::now());
        if opacity <= 0.0 {
            self.toast = None;
            return;
        }
        let fg = color_of(self.theme.ui_fg);
        let count = self.page_count();
        let screen = ctx.screen_rect();
        let dx = toast_offset(self.zoom_slider_rect, screen.center().x);
        egui::Area::new(egui::Id::new("page_toast"))
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(dx, -(BOTTOM_BAR_HEIGHT + 12.0)),
            )
            .show(ctx, |ui| {
                ui.set_max_width(screen.width() / 3.0);
                egui::Frame::default()
                    .fill(color_of(self.theme.ui_bg).gamma_multiply(opacity))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        fg.gamma_multiply(0.25 * opacity),
                    ))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("Page {} of {}", page + 1, count))
                                    .font(egui::FontId::proportional(13.0))
                                    .color(fg.gamma_multiply(opacity)),
                            )
                            .truncate(),
                        );
                    });
            });
    }

    /// Unobtrusive "Opening <name>…" line under the nav pill while an open
    /// is in flight: the previous document stays fully visible (the welcome
    /// screen for a first open) and the notice vanishes once the payload
    /// applies or fails.
    fn show_opening_notice(&self, ctx: &egui::Context) {
        let Some(job) = self.open_job.as_ref() else {
            return;
        };
        let text = format!("Opening {}…", filename_of(&job.path));
        egui::Area::new(egui::Id::new("opening_notice"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 44.0))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(text).weak().small());
            });
    }
}

impl ReaderApp {
    /// Handle the frame's keyboard input, dispatching through the loaded
    /// keybind map (`keybinds.json`). Plain-key actions stay gated on
    /// keyboard freedom — no text widget holding focus — while the
    /// ctrl-style ones match everywhere, exactly like their pre-keybind
    /// hardcoded arms.
    fn handle_input(&mut self, ctx: &egui::Context) {
        let keyboard_free = !ctx.wants_keyboard_input();
        ctx.input(|input| {
            for event in &input.events {
                let egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } = *event
                else {
                    continue;
                };
                let Some(action) = self.keybinds.action_for(key, modifiers) else {
                    continue;
                };
                match action {
                    Action::OpenFile => self.open_dialog(),
                    Action::SaveState => self.save_state(),
                    Action::ToggleSidebar => self.sidebar_open = !self.sidebar_open,
                    Action::Search => {
                        self.sidebar_open = true;
                        self.section = SidebarSection::Search;
                        self.focus_search = true;
                    }
                    Action::GoToPage if keyboard_free && self.page_count() > 0 => {
                        self.page_jump = Some((self.session.page + 1).to_string());
                        self.page_jump_focus = true;
                        self.jump_invalid = false;
                    }
                    Action::EditTheme => {
                        if self.editor.is_some() {
                            self.editor = None;
                        } else {
                            self.open_theme_editor();
                        }
                    }
                    Action::KeybindsWindow => self.shortcuts_open = true,
                    // Document zoom keys are scoped to the canvas: the theme
                    // editor owns its own font-zoom keys.
                    Action::ZoomIn
                        if keyboard_free && self.editor.is_none() && self.page_count() > 0 =>
                    {
                        self.zoom_step(5);
                    }
                    Action::ZoomOut
                        if keyboard_free && self.editor.is_none() && self.page_count() > 0 =>
                    {
                        self.zoom_step(-5);
                    }
                    Action::FitWidth
                        if keyboard_free && self.editor.is_none() && self.page_count() > 0 =>
                    {
                        self.zoom_fit_width();
                    }
                    Action::CycleTheme if keyboard_free => self.cycle_theme(),
                    Action::Bookmark if keyboard_free && self.page_count() > 0 => {
                        self.note_engagement();
                        self.session.toggle_bookmark(self.session.page);
                    }
                    Action::Quit if keyboard_free => {
                        self.save_state();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Action::FocusMode => self.focus_mode = !self.focus_mode,
                    // Escape handled by an inline text field (page jump,
                    // bookmark rename) this frame is already consumed.
                    Action::CloseOverlay if !self.escape_consumed => {
                        if self.about_open {
                            self.about_open = false;
                        } else if self.editor.is_some() {
                            self.editor = None;
                        } else if self.info_open {
                            self.info_open = false;
                        } else if self.shortcuts_open {
                            self.shortcuts_open = false;
                        } else if self.focus_mode {
                            self.focus_mode = false;
                        } else if self.sidebar_open
                            && self.section == SidebarSection::Search
                            && !self.search_query.is_empty()
                        {
                            self.cancel_search();
                            self.search_query.clear();
                            // Clearing via Esc must hand plain keys back to
                            // the dispatcher, not leave them landing in the
                            // still-focused field.
                            if let Some(focused) = ctx.memory(|mem| mem.focused()) {
                                ctx.memory_mut(|mem| mem.surrender_focus(focused));
                            }
                        }
                    }
                    Action::PrevPage if keyboard_free => {
                        self.goto_page(self.session.page.saturating_sub(1), None);
                    }
                    Action::NextPage if keyboard_free => {
                        self.goto_page(self.session.page + 1, None)
                    }
                    _ => {}
                }
            }
            // Pinch-zoom gesture (trackpad / touchscreen) and ctrl+scroll
            // report a multiplicative delta anchored at the viewport center,
            // mirroring the keyboard steps' percent-switch semantics.
            let pinch = input.zoom_delta();
            if keyboard_free && pinch != 1.0 && self.editor.is_none() && self.page_count() > 0 {
                self.set_zoom_percent(f32::from(self.zoom_pct) * pinch);
            }
        });
    }
}

/// Flat chrome: borderless idle/hover widgets, transparent idle fill — the
/// mockup's borderless icon look. Spacing is roomier than egui's defaults
/// without crowding the 640 px minimum window width.
fn chrome_style(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing.x = 10.0;
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.bg_stroke = egui::Stroke::NONE;
    widgets.hovered.bg_stroke = egui::Stroke::NONE;
    widgets.active.bg_stroke = egui::Stroke::NONE;
    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
}

/// Popup menu row: optional icon, label, optional right-aligned shortcut.
fn menu_item(
    ui: &mut egui::Ui,
    icons: &mut IconRender,
    icon: Option<Icon>,
    text: &str,
    shortcut: Option<&str>,
    fg: egui::Color32,
) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    if let Some(icon) = icon {
        icons.paint_at(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 16.0, rect.center().y),
                egui::vec2(16.0, 16.0),
            ),
            icon,
            fg,
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 34.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(14.0),
        fg,
    );
    if let Some(shortcut) = shortcut {
        ui.painter().text(
            egui::pos2(rect.right() - 8.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            egui::FontId::proportional(12.0),
            fg.gamma_multiply(0.55),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn color_of(color: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a())
}

/// The theme as painted on the big chrome masses (rail, sidebar, central
/// panel): accent-retinted so the chosen accent seeps there too, not just
/// into the egui visuals the bars and windows use.
fn tinted_chrome(theme: &Theme) -> Theme {
    retint(theme, theme.accent)
}

/// The canvas surround: the accent-tinted chrome pulled darker.
fn canvas_surface(theme: &Theme) -> egui::Color32 {
    color_of(canvas_bg(&tinted_chrome(theme)))
}

/// Restrict a saved-theme name to the filename charset; `None` when nothing
/// survives (the caller reports an inline error).
fn sanitize_theme_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .collect();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// The custom-themes directory: `<config_dir>/themes`.
fn themes_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("themes"))
}

/// What became of one frame's page-jump input field.
enum JumpOutcome {
    /// Keep editing.
    Idle,
    /// Enter on a valid page number; the payload is zero-based.
    Commit(usize),
    /// Esc or focus lost — restore the plain counter.
    Cancel,
}

/// Turn a 1-based page string into a zero-based page index, or `None` when it
/// is not a whole number inside `1..=count`.
fn validate_jump(text: &str, count: usize) -> Option<usize> {
    let Ok(page) = text.trim().parse::<usize>() else {
        return None;
    };
    if page == 0 || page > count {
        return None;
    }
    Some(page - 1)
}

/// Neighbor hit page for cyclic stepping: next takes the first hit page
/// strictly ahead of `current` (wrapping to the first hit), previous the
/// last one behind (wrapping to the final hit).
fn cycle_hit_page(hits: &[SearchHit], current: usize, dir: isize) -> usize {
    if dir > 0 {
        hits.iter()
            .find(|hit| hit.page > current)
            .unwrap_or(&hits[0])
            .page
    } else {
        hits.iter()
            .rev()
            .find(|hit| hit.page < current)
            .unwrap_or_else(|| hits.last().expect("caller checked non-empty"))
            .page
    }
}

/// ☀ or 🌙 by the UI background's luma — an icon for what is active now, not
/// a toggle promise (design spec §21).
fn theme_icon(theme: &Theme) -> Icon {
    let bg = theme.ui_bg;
    // Integer Rec.601, matching candi-theme's recolor pass.
    let luma = (77u32 * u32::from(bg.r()) + 151 * u32::from(bg.g()) + 28 * u32::from(bg.b())) >> 8;
    if luma >= 128 { Icon::Sun } else { Icon::Moon }
}

/// Shorten `text` with an ellipsis so it measures at most `max_w` wide
/// according to `width_of`; unchanged when it already fits.
fn truncate_to_width(width_of: &dyn Fn(&str) -> f32, text: &str, max_w: f32) -> String {
    if width_of(text) <= max_w {
        return text.to_owned();
    }
    let budget = max_w - width_of("…");
    let mut kept = String::new();
    for c in text.chars() {
        let mut candidate = kept.clone();
        candidate.push(c);
        if width_of(&candidate) > budget {
            break;
        }
        kept = candidate;
    }
    format!("{kept}…")
}

/// Leading offset and slider width that center the −/%/+ cluster plus slider
/// inside a `container_w`-wide column. The slider gives up width before the
/// cluster can overflow the bottom-bar third.
fn zoom_centering(container_w: f32, cluster_w: f32) -> (f32, f32) {
    let slider_w = (container_w - cluster_w).clamp(24.0, 160.0);
    let lead = ((container_w - cluster_w - slider_w) / 2.0).max(0.0);
    (lead, slider_w)
}

/// What the center pane shows this frame. The theme editor wins over
/// everything; an open failure replaces the welcome state; runtime errors on
/// a live document stay a banner over the canvas.
#[derive(Debug, PartialEq, Eq)]
enum CenterPane {
    Editor,
    Canvas,
    OpenError,
    Empty,
}

fn center_pane(editor_open: bool, has_document: bool, has_error: bool) -> CenterPane {
    if editor_open {
        CenterPane::Editor
    } else if has_document {
        CenterPane::Canvas
    } else if has_error {
        CenterPane::OpenError
    } else {
        CenterPane::Empty
    }
}

/// Top offset that vertically centers a `block`-tall group in an
/// `available`-tall region (clamped when the block overflows).
fn centered_top_offset(available: f32, block: f32) -> f32 {
    ((available - block) / 2.0).max(0.0)
}

/// Measured height of the welcome block: icon, title, hints, CTA, the
/// quick-start panel until onboarding is done, and the recents list —
/// widget heights plus one item-spacing per allocation, the model
/// [`centered_top_offset`] centers against.
fn welcome_block_height(ui: &egui::Ui, recents: usize, onboarding: bool) -> f32 {
    let body = ui.text_style_height(&egui::TextStyle::Body);
    let small = ui.text_style_height(&egui::TextStyle::Small);
    let title = ui
        .painter()
        .layout_no_wrap(
            "No file open".to_owned(),
            egui::FontId::proportional(18.0),
            egui::Color32::WHITE,
        )
        .rect
        .height();
    let spacing = ui.spacing().item_spacing.y;
    let mut allocations = 9.0;
    let mut h = 72.0 + 14.0 + title + 4.0 + body + 18.0 + 36.0 + 10.0 + small;
    if onboarding {
        let button =
            (small + 2.0 * ui.spacing().button_padding.y).max(ui.spacing().interact_size.y);
        allocations += 7.0;
        h += 26.0 + small + 2.0 + small + small + 6.0 + button;
    }
    if recents > 0 {
        allocations += 3.0 + recents as f32;
        h += 26.0 + small + 2.0 + recents as f32 * body;
    }
    h + allocations * spacing
}

/// Short cause for the open-failure card: the first line of the backend
/// error. The full text stays behind the Details disclosure.
fn humanize_reason(raw: &str) -> &str {
    raw.lines().next().unwrap_or(raw)
}

/// The inline page-jump field shown in place of the `n / N` counter. Enter
/// commits a valid page, an invalid entry tints red and keeps editing, and
/// Esc or clicking away cancels back to the counter.
fn jump_input(
    ui: &mut egui::Ui,
    buffer: &mut String,
    count: usize,
    focus: &mut bool,
    invalid: &mut bool,
    guard: &mut FocusGuard,
    escape: &mut bool,
) -> JumpOutcome {
    let mut edit = egui::TextEdit::singleline(buffer)
        .desired_width(64.0)
        .font(egui::TextStyle::Monospace);
    if *invalid {
        edit = edit.text_color(ERROR_RED);
    }
    let field = ui.add(edit);
    guard.register(&field);
    if *focus {
        guard.request(&field);
        *focus = false;
    }
    if !field.lost_focus() {
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            *escape = true;
            return JumpOutcome::Cancel;
        }
        if field.changed() {
            *invalid = false;
        }
        return JumpOutcome::Idle;
    }
    // Single-line fields release focus on Enter; anything else that stole
    // focus counts as a cancel.
    if ui.input(|i| i.key_pressed(Key::Enter)) {
        match validate_jump(buffer, count) {
            Some(page) => return JumpOutcome::Commit(page),
            None => {
                *invalid = true;
                field.request_focus();
                return JumpOutcome::Idle;
            }
        }
    }
    JumpOutcome::Cancel
}

/// One contents row spanning the panel's full width exactly like the search
/// and bookmark rows: title indented 12 pt per outline level, truncated.
/// Rows containing the reading position are accent-tinted (design spec
/// §12/§24) and hover fills underlay the row like every other list.
fn toc_row_ui(
    ui: &mut egui::Ui,
    row: &TocRow,
    active: bool,
    accent: egui::Color32,
) -> egui::Response {
    let full_w = ui.available_width();
    let indent = INDENT_PER_LEVEL * row.depth as f32;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(indent);
        let mut title = egui::RichText::new(&row.title);
        if active {
            title = title.color(accent);
        }
        let title_w = (full_w - indent).max(40.0);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(title_w, 20.0), egui::Sense::click());
        if resp.hovered() {
            ui.painter()
                .rect_filled(rect.expand2(egui::vec2(4.0, 2.0)), 4.0, row_hover_fill(ui));
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
            | ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(rect).layout(
                    egui::Layout::left_to_right(egui::Align::Center)
                        .with_main_align(egui::Align::LEFT)
                        .with_main_justify(true),
                ),
                |ui| ui.add(egui::Label::new(title).truncate().selectable(false)),
            )
            .inner
    })
    .inner
}

/// Subtle list-row hover fill: a whisper of the foreground so it reads on
/// both light and dark chrome without competing with the accent selection.
fn row_hover_fill(ui: &egui::Ui) -> egui::Color32 {
    let fg = ui.visuals().text_color();
    egui::Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), 20)
}

/// Clickable list row (recents, bookmarks, search hits): the label truncates
/// instead of wrapping and a hover fill paints under the text. `width` is the
/// hover strip width — full column for sidebar lists, text-hugging for the
/// centered welcome recents.
fn click_row(ui: &mut egui::Ui, text: &str, color: egui::Color32, width: f32) -> egui::Response {
    let height = ui.text_style_height(&egui::TextStyle::Body);
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(width.max(40.0), height), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, row_hover_fill(ui));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let shown = truncate_to_width(
        &|s: &str| {
            ui.painter()
                .layout_no_wrap(s.to_owned(), font.clone(), color)
                .rect
                .width()
        },
        text,
        rect.width() - 8.0,
    );
    ui.painter().text(
        rect.left_center(),
        egui::Align2::LEFT_CENTER,
        &shown,
        font,
        color,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Width of an [`IconRender::button`]: the shrunk icon plus button padding.
fn icon_button_width(ui: &egui::Ui, side: f32) -> f32 {
    (side - 10.0).max(8.0) + 2.0 * ui.spacing().button_padding.x
}

fn uv_unit_rect() -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
}

/// The page and depth-fraction under content-y `center_y` — what a relayout
/// (zoom step, flow switch) must keep under the viewport center so the
/// content does not slide away. Between-page positions resolve to the
/// earlier row via [`Layout::page_at`].
fn center_anchor(layout: &Layout, center_y: f32) -> Option<(usize, f32)> {
    let page = layout.page_at(center_y)?;
    let rect = layout.rects.get(page)?;
    let frac = ((center_y - rect.y) / rect.h).clamp(0.0, 1.0);
    Some((page, frac))
}

/// Absolute scroll offset putting `anchor`'s content point at the viewport
/// center of height `viewport_h`, clamped to the scrollable span.
fn anchored_offset(layout: &Layout, anchor: (usize, f32), viewport_h: f32) -> Option<f32> {
    let rect = layout.rects.get(anchor.0)?;
    let y = rect.y + anchor.1 * rect.h - viewport_h * 0.5;
    Some(y.clamp(0.0, (layout.total_height - viewport_h).max(0.0)))
}

fn filename_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document")
        .to_owned()
}

/// Map theme tokens onto egui visuals; called whenever the theme changes.
/// The chrome surfaces are first retinted toward the accent so a chosen
/// accent seeps into the surrounding UI while pages stay clean.
fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let theme = retint(theme, theme.accent);
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
        // Fill the rail from its start to the handle so the handle visibly
        // sits ON the track; without it, rail and handle share one fill and
        // read as a detached dot.
        visuals.slider_trailing_fill = true;

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
        // Window title bars take their fill from `widgets.open`, which egui
        // leaves at the dark-mode default; map it to a themed surface.
        widgets.open.weak_bg_fill = color_of(theme.panel_bg);
    });
}

impl eframe::App for ReaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.applied_theme != self.theme.name {
            apply_theme(ctx, &self.theme);
            self.applied_theme.clone_from(&self.theme.name);
        }
        // Release focus parked on chrome widgets by egui's Tab/arrow
        // navigation before this frame's dispatcher runs, so plain keys
        // reach their bindings again. Runs against last frame's editables —
        // the registry is reset below and repopulated while rendering.
        if let Some(id) = transient_focus(ctx.memory(|mem| mem.focused()), &self.focus.editables) {
            ctx.memory_mut(|mem| mem.surrender_focus(id));
        }
        self.focus = FocusGuard::default();
        self.escape_consumed = false;

        self.collect_results(ctx);
        // A file dropped onto the window opens like any other (design spec
        // §35); wrong types fail through the normal error path.
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped.iter().find_map(|f| f.path.clone()) {
            self.open_path(file);
        }
        if !self.focus_mode {
            egui::TopBottomPanel::top("top_bar")
                .exact_height(46.0)
                .show(ctx, |ui| self.top_bar(ui));
            let panel_frame = egui::Frame::default()
                .inner_margin(0.0)
                .fill(color_of(tinted_chrome(&self.theme).panel_bg));
            egui::SidePanel::left("rail")
                .exact_width(52.0)
                .resizable(false)
                .frame(panel_frame)
                .show(ctx, |ui| self.rail(ui));
            if self.sidebar_open {
                egui::SidePanel::left("panel")
                    .resizable(true)
                    .default_width((ctx.screen_rect().width() * 0.26).clamp(230.0, 310.0))
                    .min_width(230.0)
                    .max_width(400.0)
                    .show_separator_line(false)
                    .frame(panel_frame)
                    .show(ctx, |ui| self.section_panel(ui));
            }
            egui::TopBottomPanel::bottom("bottom_bar")
                .exact_height(42.0)
                .show(ctx, |ui| self.bottom_bar(ui));
        }

        let pane = center_pane(
            self.editor.is_some(),
            self.document.is_some(),
            self.error.is_some(),
        );
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(0.0).fill(match pane {
                // The canvas paints its own darker surround; every
                // other pane keeps the chrome surface.
                CenterPane::Canvas => canvas_surface(&self.theme),
                _ => color_of(tinted_chrome(&self.theme).panel_bg),
            }))
            .show(ctx, |ui| match pane {
                CenterPane::Editor => self.show_theme_editor(ui),
                CenterPane::Canvas => {
                    if let Some(error) = self.error.clone() {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(ERROR_RED, error);
                            if ui.small_button("dismiss").clicked() {
                                self.error = None;
                            }
                        });
                        ui.separator();
                    }
                    self.show_canvas(ui);
                }
                CenterPane::OpenError => self.open_error_view(ui),
                CenterPane::Empty => self.empty_state(ui),
            });

        self.show_page_toast(ctx);
        self.show_opening_notice(ctx);
        self.about_window(ctx);
        self.info_window(ctx);
        self.shortcuts_window(ctx);
        self.handle_input(ctx);
        self.poll_search();
        self.poll_dialog();
        self.poll_open();

        // A click or wheel scroll outside the text fields releases their
        // focus, so plain keys reach the keybind dispatcher again.
        let focus = ctx.memory(|mem| mem.focused());
        let (click_pos, scroll_pos) = ctx.input(|input| {
            let pos = input.pointer.interact_pos();
            let clicked = input.pointer.primary_clicked().then_some(pos).flatten();
            let scrolled = (input.raw_scroll_delta != egui::Vec2::ZERO)
                .then_some(pos)
                .flatten();
            (clicked, scrolled)
        });
        if let Some(focused) = focus
            && should_release_focus(
                click_pos,
                scroll_pos,
                Some(focused),
                &self.focus.editables,
                self.focus.requested,
            )
        {
            ctx.memory_mut(|mem| mem.surrender_focus(focused));
        }

        // Completed renders must wake the UI even when no input arrives;
        // poll at a fixed cadence while anything is outstanding. Retry
        // backoffs wake exactly when they come due (elapsed ones are
        // requeued only if their page turns visible again), streaming
        // searches, open dialogs, and opens poll at the fixed cadence, and
        // the toast repaints while it holds and fades. While the window is
        // unfocused nothing schedules: workers keep completing, and their
        // results surface on the next event-driven frame after refocus.
        let now = Instant::now();
        if let Some(wait) = should_schedule_repaint(
            ctx.input(|input| input.focused),
            !self.pending.is_empty(),
            self.failed
                .values()
                .filter_map(|state| state.next_retry)
                .filter(|due| *due > now)
                .min()
                .map(|due| due - now),
            self.search_job.is_some() || self.dialog_result.is_some() || self.open_job.is_some(),
            self.toast.map(|(_, shown)| {
                let elapsed = now.saturating_duration_since(shown);
                if elapsed < TOAST_HOLD {
                    TOAST_HOLD - elapsed
                } else {
                    TOAST_FADE / 8
                }
            }),
        ) {
            ctx.request_repaint_after(wait);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.session_corrupt {
            return;
        }
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
    use std::time::Instant;

    use candi_pdf::Backend as _;
    use candi_pdf::stub::StubBackend;
    use eframe::App as _;

    fn builtin_theme(name: &str) -> Theme {
        builtin(name).unwrap_or_else(|| panic!("{name} must exist"))
    }

    fn test_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "candi-gui-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A bare app with no config persistence (`config_path: None`) so tests
    /// never touch the developer's real config.
    fn test_app(backend: BackendKind) -> ReaderApp {
        ReaderApp {
            backend,
            path: None,
            filename: String::new(),
            document: None,
            session: SessionState::new(1),
            session_corrupt: false,
            theme: builtin_theme(DEFAULT_THEME),
            applied_theme: String::new(),
            config: Prefs::default(),
            config_path: None,
            cache: ImageCache::new(DEFAULT_BUDGET_BYTES),
            recolored: ImageCache::new(RECOLORED_BUDGET_BYTES),
            pipeline: None,
            pending: HashSet::new(),
            failed: HashMap::new(),
            failed_scale_q: 0,
            textures: HashMap::new(),
            icons: IconRender::default(),
            layout: Layout::default(),
            layout_key: None,
            page_sizes: Vec::new(),
            zoom_pct: layout::MIN_ZOOM_PERCENT,
            fit_page: false,
            flow: Flow::Continuous,
            canvas_center_y: 0.0,
            scroll_anchor: None,
            debug_relayout: None,
            debug_picker: false,
            sidebar_open: false,
            section: SidebarSection::Contents,
            focus: FocusGuard::default(),
            toast: None,
            zoom_slider_rect: None,
            last_toasted_page: None,
            toc_follow: None,
            ui_scale: 1.0,
            base_ppp: 1.0,
            focus_search: false,
            focus_mode: false,
            page_jump: None,
            page_jump_focus: false,
            renaming: None,
            rename_buffer: String::new(),
            rename_focus: false,
            jump_invalid: false,
            search_query: String::new(),
            search_hits: None,
            search_job: None,
            open_job: None,
            escape_consumed: false,
            dialog_result: None,
            highlight: None,
            toc_rows: Vec::new(),
            about_open: false,
            info_open: false,
            shortcuts_open: false,
            shortcut_filter: String::new(),
            keybinds: Keybinds::load_or_init(None),
            custom_themes: Vec::new(),
            theme_status: None,
            editor: None,
            restore_frac: None,
            pending_scroll: None,
            primed: true,
            error: None,
        }
    }

    fn fixture_copy(name: &str, dir: &Path, file_name: &str) -> PathBuf {
        let pdf = dir.join(file_name);
        fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../candi-pdf/tests/fixtures")
                .join(name),
            &pdf,
        )
        .unwrap();
        pdf
    }

    /// Wait for the in-flight open to land and apply.
    fn drain_open(app: &mut ReaderApp) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.open_job.is_some() {
            app.poll_open();
            assert!(Instant::now() < deadline, "open did not finish in time");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn open_marks_onboarding_done_and_persists() {
        let dir = test_dir("onboarding-open");
        let pdf = fixture_copy("tiny.pdf", &dir, "tiny.pdf");
        let config = dir.join("config.toml");
        let mut app = test_app(BackendKind::Mupdf);
        app.config_path = Some(config.clone());
        assert!(!app.config.onboarding_done);
        app.open_path(pdf);
        drain_open(&mut app);
        assert!(app.document.is_some());
        assert!(app.config.onboarding_done, "first successful open marks it");
        assert!(
            fs::read_to_string(&config)
                .unwrap()
                .contains("onboarding_done = true"),
            "the flag persists immediately"
        );
        assert!(load_prefs(&config).onboarding_done);
    }

    #[test]
    fn dismiss_onboarding_persists_immediately() {
        let dir = test_dir("onboarding-dismiss");
        let config = dir.join("config.toml");
        let mut app = test_app(BackendKind::Mupdf);
        app.config_path = Some(config.clone());
        app.dismiss_onboarding();
        assert!(app.config.onboarding_done);
        assert!(load_prefs(&config).onboarding_done, "dismissal persists");
    }

    #[test]
    fn corrupt_sidecar_blocks_saving_until_engagement() {
        let dir = test_dir("corrupt-sidecar");
        let pdf = fixture_copy("tiny.pdf", &dir, "tiny.pdf");
        let sidecar = dir.join("tiny.pdf.candi.toml");
        let corrupt = "not valid {{{ toml";
        fs::write(&sidecar, corrupt).unwrap();

        let mut app = test_app(BackendKind::Mupdf);
        app.open_path(pdf.clone());
        drain_open(&mut app);
        assert!(app.session_corrupt, "the corrupt load must set the flag");
        assert_eq!(app.error.as_deref(), Some(SESSION_CORRUPT_BANNER));

        app.on_exit(None);
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            corrupt,
            "no save while the flag is set"
        );

        app.goto_page(1, None);
        assert!(!app.session_corrupt, "navigation is genuine engagement");
        app.on_exit(None);
        assert_ne!(
            fs::read_to_string(&sidecar).unwrap(),
            corrupt,
            "engagement authorizes the save"
        );
        match candi_core::load_session(&pdf) {
            // tiny.pdf is single-page: the requested page 1 clamps to 0.
            Ok(candi_core::SessionLoad::Loaded(session)) => assert_eq!(session.page, 0),
            other => panic!("expected a saved session, got {other:?}"),
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failed_open_preserves_the_previous_sidebar() {
        let mut app = test_app(BackendKind::Mupdf);
        app.toc_rows = vec![TocRow {
            title: "Chapter".into(),
            page: 1,
            dest_top: Some(0.0),
            depth: 0,
        }];
        app.open_path(test_dir("failed-open").join("missing.pdf"));
        drain_open(&mut app);
        assert!(
            !app.toc_rows.is_empty(),
            "a failed open must not gut the live document's sidebar"
        );
        assert!(app.error.is_some());
    }

    #[test]
    fn successful_open_applies_the_whole_payload_off_thread() {
        let dir = test_dir("async-open");
        let pdf = fixture_copy("tiny.pdf", &dir, "tiny.pdf");
        let mut app = test_app(BackendKind::Mupdf);
        app.open_path(pdf.clone());
        assert!(
            app.document.is_none() && app.open_job.is_some(),
            "the open runs on its worker before the payload applies"
        );
        drain_open(&mut app);
        assert_eq!(app.path.as_deref(), Some(pdf.as_path()));
        assert_eq!(app.filename, "tiny.pdf");
        assert_eq!(
            app.document.as_ref().map(|doc| doc.page_count()),
            Some(1),
            "tiny.pdf is single-page"
        );
        assert!(
            !app.page_sizes.is_empty(),
            "page sizes land with the payload, not on the UI thread"
        );
        assert!(!app.primed, "the canvas primes on the next UI frame");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_newer_open_supersedes_the_in_flight_one() {
        let dir = test_dir("supersede");
        let first = fixture_copy("tiny.pdf", &dir, "first.pdf");
        let second = fixture_copy("tiny.pdf", &dir, "second.pdf");
        let mut app = test_app(BackendKind::Mupdf);
        app.open_path(first);
        app.open_path(second.clone());
        drain_open(&mut app);
        assert_eq!(app.path.as_deref(), Some(second.as_path()));
        assert_eq!(app.filename, "second.pdf");
        assert!(app.open_job.is_none());
        assert_eq!(
            app.last_toasted_page, None,
            "a fresh document forgets the last toasted page"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failed_theme_delete_keeps_the_registry_entry() {
        let mut app = test_app(BackendKind::Mupdf);
        let mut theme = builtin_theme("Dark");
        theme.name = "Ghost".into();
        let gone = test_dir("theme-delete").join("Ghost.yaml");
        app.custom_themes.push((theme.clone(), gone));
        app.delete_custom_theme("Ghost");
        assert_eq!(
            app.custom_themes.len(),
            1,
            "a failed delete keeps the entry"
        );
        assert!(
            app.theme_status.is_some(),
            "the failure must surface inline"
        );

        // A real file deletes cleanly, clears the status, and falls back
        // when the deleted theme was active.
        let dir = test_dir("theme-delete-ok");
        let mut real = builtin_theme("Dark");
        real.name = "Real".into();
        write_file_atomically(&dir.join("Real.yaml"), &to_yaml(&real)).unwrap();
        app.custom_themes
            .push((real.clone(), dir.join("Real.yaml")));
        app.theme = real;
        app.delete_custom_theme("Real");
        assert_eq!(app.custom_themes.len(), 1);
        assert_eq!(app.theme_status, None);
        assert!(!dir.join("Real.yaml").exists());
        assert_eq!(app.theme.name, DEFAULT_THEME, "active theme fell back");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn oversized_theme_files_are_skipped_unread() {
        assert!(!oversized_theme(MAX_THEME_BYTES));
        assert!(oversized_theme(MAX_THEME_BYTES + 1));

        let dir = test_dir("oversize");
        let mut mint = builtin_theme("Dark");
        mint.name = "Mint".into();
        write_file_atomically(&dir.join("Mint.yaml"), &to_yaml(&mint)).unwrap();
        // A valid theme whose file exceeds the cap: skipped before reading,
        // so it must not appear even though its name matches the stem.
        let big = format!("name: Big\n{}\n", "# padding\n".repeat(30_000));
        assert!(big.len() as u64 > MAX_THEME_BYTES);
        write_file_atomically(&dir.join("Big.yaml"), &big).unwrap();

        let loaded = ReaderApp::load_custom_themes(Some(&dir));
        assert_eq!(loaded.len(), 1, "{loaded:?}");
        assert_eq!(loaded[0].0.name, "Mint");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sanitize_keeps_only_the_filename_charset() {
        assert_eq!(
            sanitize_theme_name("My Cool Theme!"),
            Some("My Cool Theme".to_owned())
        );
        assert_eq!(sanitize_theme_name("a/b\\c"), Some("abc".to_owned()));
        assert_eq!(sanitize_theme_name("!!!"), None);
        assert_eq!(sanitize_theme_name(""), None);
    }

    #[test]
    fn picker_names_fit_their_budget_or_gain_an_ellipsis() {
        let width_of = |s: &str| s.chars().count() as f32 * 10.0;
        assert_eq!(
            truncate_to_width(&width_of, "Dark", 57.0),
            "Dark",
            "short names pass through"
        );
        let long = truncate_to_width(&width_of, "Sepia But Warmer Still", 57.0);
        assert!(long.ends_with('…'));
        // The result plus ellipsis fits the budget exactly (4 kept chars).
        assert_eq!(long, "Sepi…");
        assert!(width_of(&long) <= 57.0);
    }

    #[test]
    fn a_tiny_budget_collapses_to_a_bare_ellipsis() {
        let width_of = |s: &str| s.chars().count() as f32 * 10.0;
        assert_eq!(truncate_to_width(&width_of, "Warm Dark", 5.0), "…");
    }

    #[test]
    fn zoom_cluster_centers_and_never_overflows_its_column() {
        // Wide column: full slider, symmetric lead.
        let (lead, slider_w) = zoom_centering(400.0, 140.0);
        assert_eq!(slider_w, 160.0);
        assert_eq!(lead, (400.0 - 140.0 - 160.0) / 2.0);
        // Narrow column: slider shrinks first; it bottoms out at 24 only
        // when the column drops below cluster + 24.
        let (lead, slider_w) = zoom_centering(213.0, 180.0);
        assert_eq!((lead, slider_w), (0.0, 33.0));
        let (lead, slider_w) = zoom_centering(198.0, 180.0);
        assert_eq!((lead, slider_w), (0.0, 24.0));
        // Degenerate: no negative offsets.
        let (lead, slider_w) = zoom_centering(100.0, 180.0);
        assert_eq!((lead, slider_w), (0.0, 24.0));
    }

    #[test]
    fn centered_top_offset_clamps_at_zero() {
        assert_eq!(centered_top_offset(500.0, 300.0), 100.0);
        assert_eq!(centered_top_offset(100.0, 300.0), 0.0);
    }

    #[test]
    fn welcome_block_grows_with_recents() {
        let ctx = egui::Context::default();
        let measure = |recents: usize, onboarding: bool| -> f32 {
            let height = std::cell::Cell::new(0.0_f32);
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    height.set(welcome_block_height(ui, recents, onboarding));
                });
            });
            height.get()
        };
        let none = measure(0, false);
        let three = measure(3, false);
        assert!(none > 0.0);
        assert!(three > none + 3.0, "each recent row adds its height");
        assert!(
            measure(0, true) > none + 3.0,
            "the quick-start panel adds its height"
        );
    }

    #[test]
    fn custom_theme_roundtrip_through_a_temp_dir() {
        let dir = test_dir("roundtrip");
        let mut mint = builtin_theme("Dark");
        mint.name = "Mint".into();
        write_file_atomically(&dir.join("Mint.yaml"), &to_yaml(&mint)).unwrap();
        // Embedded name must match the stem to load.
        let mismatched = builtin_theme("Light");
        write_file_atomically(&dir.join("Mismatch.yaml"), &to_yaml(&mismatched)).unwrap();
        // Unparsable files are skipped with a warning.
        write_file_atomically(&dir.join("Broken.yaml"), "name: [").unwrap();

        let loaded = ReaderApp::load_custom_themes(Some(&dir));
        assert_eq!(loaded.len(), 1, "{loaded:?}");
        assert_eq!(loaded[0].0.name, "Mint");

        // Custom names resolve; unknown ones do not.
        assert_eq!(
            ReaderApp::resolve_theme("Mint", &loaded).map(|t| t.name),
            Some("Mint".into())
        );
        assert_eq!(ReaderApp::resolve_theme("Nope", &loaded), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_themes_dir_loads_nothing() {
        assert!(ReaderApp::load_custom_themes(None).is_empty());
        assert!(ReaderApp::load_custom_themes(Some(&test_dir("absent").join("ghost"))).is_empty());
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

    #[test]
    fn jump_accepts_pages_inside_the_document() {
        assert_eq!(validate_jump("1", 672), Some(0));
        assert_eq!(validate_jump("17", 672), Some(16));
        assert_eq!(validate_jump("672", 672), Some(671), "last page");
    }

    #[test]
    fn jump_trims_surrounding_whitespace() {
        assert_eq!(validate_jump(" 42 ", 100), Some(41));
    }

    #[test]
    fn jump_rejects_zero_out_of_range_and_garbage() {
        let count = 5;
        assert_eq!(validate_jump("", count), None);
        assert_eq!(validate_jump("abc", count), None);
        assert_eq!(validate_jump("0", count), None, "pages are 1-based");
        assert_eq!(validate_jump("6", count), None);
        assert_eq!(validate_jump("-1", count), None);
        assert_eq!(
            validate_jump("99999999999999999999999", count),
            None,
            "parse overflow"
        );
    }

    #[test]
    fn theme_icon_follows_the_ui_background_luma() {
        for name in BUILTIN_NAMES {
            let icon = theme_icon(&builtin_theme(name));
            assert!(
                icon == Icon::Sun || icon == Icon::Moon,
                "{name} produced {icon:?}"
            );
        }
        assert_eq!(theme_icon(&builtin_theme("Light")), Icon::Sun);
        assert_eq!(
            theme_icon(&builtin_theme("Solarized Light")),
            Icon::Sun,
            "solarized keeps light chrome like Light"
        );
        assert_eq!(
            theme_icon(&builtin_theme("Sepia")),
            Icon::Moon,
            "sepia warms the page, its chrome stays dark"
        );
        assert_eq!(theme_icon(&builtin_theme("Dark")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("Warm Dark")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("True Dark")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("Cyberpunk")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("Catppuccin")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("Nord")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("Dracula")), Icon::Moon);
        assert_eq!(theme_icon(&builtin_theme("Gruvbox Dark")), Icon::Moon);
    }

    #[test]
    fn center_pane_precedence_is_editor_then_document_then_error() {
        assert_eq!(center_pane(true, true, true), CenterPane::Editor);
        assert_eq!(center_pane(true, false, false), CenterPane::Editor);
        assert_eq!(center_pane(false, true, false), CenterPane::Canvas);
        // A runtime error on a live document keeps the canvas (banner).
        assert_eq!(center_pane(false, true, true), CenterPane::Canvas);
        assert_eq!(center_pane(false, false, true), CenterPane::OpenError);
        assert_eq!(center_pane(false, false, false), CenterPane::Empty);
    }

    #[test]
    fn humanized_reason_takes_the_first_error_line() {
        assert_eq!(
            humanize_reason("mupdf: cannot open\n  cause: nope"),
            "mupdf: cannot open"
        );
        assert_eq!(humanize_reason("single line"), "single line");
        assert_eq!(humanize_reason(""), "");
    }

    fn hits(pages: &[usize]) -> Vec<SearchHit> {
        pages
            .iter()
            .map(|&page| SearchHit {
                page,
                snippet: String::new(),
            })
            .collect()
    }

    #[test]
    fn search_stepping_walks_hits_and_cycles_at_the_ends() {
        let sample = hits(&[2, 5, 9]);
        assert_eq!(cycle_hit_page(&sample, 1, 1), 2);
        assert_eq!(cycle_hit_page(&sample, 5, 1), 9);
        assert_eq!(cycle_hit_page(&sample, 8, -1), 5);
        assert_eq!(cycle_hit_page(&sample, 7, -1), 5);
        assert_eq!(cycle_hit_page(&sample, 9, 1), 2, "next wraps past the end");
        assert_eq!(
            cycle_hit_page(&sample, 0, -1),
            9,
            "previous wraps before the start"
        );
        assert_eq!(
            cycle_hit_page(&hits(&[4]), 4, 1),
            4,
            "single-page lists step in place"
        );
    }

    fn letters(count: usize) -> Vec<(f32, f32)> {
        vec![(612.0, 792.0); count]
    }

    #[test]
    fn center_anchor_resolves_to_the_row_first_page_and_depth_fraction() {
        let layout = Layout::build(&letters(5), ZoomMode::Percent(100), 800.0, Flow::Continuous);
        let mid = |page: usize, frac: f32| {
            let r = layout.rects[page];
            r.y + frac * r.h
        };
        // Halfway down page 3.
        assert_eq!(center_anchor(&layout, mid(3, 0.5)), Some((3, 0.5)));
        // A between-row gap resolves to the earlier row's bottom edge.
        let gap_y = layout.rects[1].y + 792.0 + GAP / 2.0;
        assert_eq!(center_anchor(&layout, gap_y), Some((1, 1.0)));
        // Positions outside the content clamp into the nearest row.
        assert_eq!(center_anchor(&layout, -50.0), Some((0, 0.0)));
        assert_eq!(center_anchor(&Layout::default(), 10.0), None);
    }

    #[test]
    fn anchored_offset_keeps_the_anchor_point_centered_across_zooms() {
        let vh = 600.0;
        let before = Layout::build(
            &letters(12),
            ZoomMode::Percent(100),
            800.0,
            Flow::Continuous,
        );
        let after = Layout::build(
            &letters(12),
            ZoomMode::Percent(150),
            800.0,
            Flow::Continuous,
        );

        let anchor =
            center_anchor(&before, before.rects[5].y + 0.25 * before.rects[5].h).expect("resolves");
        let restored =
            anchored_offset(&after, anchor, vh).expect("offset resolves for a non-empty layout");
        let new_center = restored + vh * 0.5;
        // The anchored fraction of the same page sits under the viewport
        // center again, so zooming does not move the reading position.
        assert_eq!(after.page_at(new_center), Some(5));
        let target = after.rects[5].y + anchor.1 * after.rects[5].h;
        assert!((new_center - target).abs() < 1e-3);
    }

    #[test]
    fn flow_relayout_keeps_the_anchored_row_under_the_viewport_center() {
        let before = Layout::build(&letters(9), ZoomMode::Percent(100), 800.0, Flow::Continuous);
        let anchor =
            center_anchor(&before, before.rects[4].y + 0.7 * before.rects[4].h).expect("resolves");
        let after = Layout::build(&letters(9), ZoomMode::Percent(100), 1300.0, Flow::Dual);
        let vh = 500.0;
        let restored = anchored_offset(&after, anchor, vh).expect("resolves");
        // Page 4 pairs into spread (4, 5) after switching to dual flow; its
        // depth point stays centered, so the same spread remains on screen.
        assert_eq!(after.page_at(restored + vh * 0.5), Some(4));
    }

    #[test]
    fn anchored_offset_clamps_to_the_scrollable_span() {
        let tiny = Layout::build(&letters(2), ZoomMode::Percent(100), 800.0, Flow::Continuous);
        // A viewport taller than the document cannot scroll at all.
        assert_eq!(
            anchored_offset(&tiny, (1, 1.0), tiny.total_height * 2.0),
            Some(0.0)
        );
        let last = Layout::build(
            &letters(12),
            ZoomMode::Percent(150),
            800.0,
            Flow::Continuous,
        );
        let vh = 600.0;
        assert_eq!(
            anchored_offset(&last, (11, 1.0), vh),
            Some(last.total_height - vh)
        );
        assert_eq!(anchored_offset(&last, (0, 0.0), vh), Some(0.0));
        assert_eq!(anchored_offset(&Layout::default(), (0, 0.0), vh), None);
    }

    fn key(page: usize, scale_q: u16) -> CacheKey {
        CacheKey { page, scale_q }
    }

    #[test]
    fn render_failures_back_off_then_terminate() {
        let now = Instant::now();
        let mut failed = HashMap::new();
        note_render_failure(&mut failed, key(3, 100), now, "boom".into());
        assert_eq!(failed[&key(3, 100)].attempt, 1);
        assert_eq!(
            failed[&key(3, 100)].next_retry.unwrap() - now,
            RETRY_BACKOFFS[0]
        );
        // The retry's failure builds on the recorded attempt count.
        note_render_failure(&mut failed, key(3, 100), now, "boom".into());
        assert_eq!(failed[&key(3, 100)].attempt, 2);
        assert_eq!(
            failed[&key(3, 100)].next_retry.unwrap() - now,
            RETRY_BACKOFFS[1]
        );
        note_render_failure(&mut failed, key(3, 100), now, "boom".into());
        assert_eq!(
            failed[&key(3, 100)].next_retry.unwrap() - now,
            RETRY_BACKOFFS[2]
        );
        // The fourth failure exhausts the schedule: terminal.
        note_render_failure(&mut failed, key(3, 100), now, "boom".into());
        assert_eq!(failed[&key(3, 100)].attempt, 4);
        assert_eq!(failed[&key(3, 100)].next_retry, None);
    }

    #[test]
    fn wants_render_respects_pending_and_retry_schedule() {
        let now = Instant::now();
        let mut pending = HashSet::new();
        let mut failed = HashMap::new();

        // Fresh page: render it.
        assert!(wants_render(&pending, &failed, key(0, 100), now));
        // Already queued: don't double-book.
        pending.insert(key(0, 100));
        assert!(!wants_render(&pending, &failed, key(0, 100), now));
        pending.clear();

        // A backoff still in the future waits; an elapsed one requeues.
        failed.insert(
            key(1, 100),
            FailState {
                attempt: 1,
                next_retry: Some(now + Duration::from_secs(1)),
                detail: "boom".into(),
            },
        );
        assert!(!wants_render(&pending, &failed, key(1, 100), now));
        assert!(wants_render(
            &pending,
            &failed,
            key(1, 100),
            now + Duration::from_secs(2)
        ));
        // Terminal failures only revive by click (the ledger entry removed).
        failed.insert(
            key(2, 100),
            FailState {
                attempt: 4,
                next_retry: None,
                detail: "boom".into(),
            },
        );
        assert!(!wants_render(&pending, &failed, key(2, 100), now));
    }

    #[test]
    fn zoom_change_prunes_stale_scale_state() {
        let mut pending = HashSet::from([key(0, 100), key(1, 100), key(1, 125), key(2, 125)]);
        let mut failed = HashMap::from([
            (
                key(3, 100),
                FailState {
                    attempt: 1,
                    next_retry: None,
                    detail: "old scale".into(),
                },
            ),
            (
                key(4, 125),
                FailState {
                    attempt: 2,
                    next_retry: None,
                    detail: "current scale".into(),
                },
            ),
        ]);
        prune_stale_scale(&mut pending, &mut failed, 125);
        assert_eq!(
            pending,
            HashSet::from([key(1, 125), key(2, 125)]),
            "only current-scale queue entries survive"
        );
        assert!(failed.is_empty(), "failures never outlive their scale");
    }

    #[test]
    fn queue_renders_records_keys_only_after_an_accepted_submit() {
        let doc: Arc<dyn Document> = Arc::from(StubBackend::new(3).open("x.pdf", None).unwrap());
        let pipeline = Pipeline::spawn(doc);
        let mut pending = HashSet::new();
        let wanted = vec![RenderRequest {
            page: 0,
            scale_q: 100,
            scale: 1.0,
        }];
        assert!(matches!(
            queue_renders(Some(&pipeline), &wanted, &mut pending),
            Submission::Queued
        ));
        assert_eq!(pending, HashSet::from([key(0, 100)]));

        // No renderer: nothing queues, nothing regrows.
        pending.clear();
        assert!(matches!(
            queue_renders(None, &wanted, &mut pending),
            Submission::NoRenderer
        ));
        assert!(pending.is_empty(), "a dead renderer grows no pending keys");
    }

    #[test]
    fn toast_fades_after_the_hold() {
        let shown = Instant::now();
        assert_eq!(toast_opacity(shown, shown), 1.0);
        assert_eq!(toast_opacity(shown, shown + TOAST_HOLD), 1.0);
        let mid = toast_opacity(shown, shown + TOAST_HOLD + TOAST_FADE / 2);
        assert!(mid > 0.2 && mid < 0.8, "mid-fade opacity {mid}");
        assert_eq!(
            toast_opacity(shown, shown + TOAST_HOLD + TOAST_FADE),
            0.0,
            "gone after the fade"
        );
        assert_eq!(
            toast_opacity(shown, shown + TOAST_HOLD + TOAST_FADE * 3),
            0.0
        );
    }

    #[test]
    fn toasts_fire_once_per_page_change() {
        let mut last = None;
        assert!(should_toast(last, 1), "a fresh document toasts its page");
        last = Some(1);
        // The toast for page 1 expired long ago; hovering the same page
        // must not resurrect it.
        assert!(!should_toast(last, 1));
        assert!(should_toast(last, 2), "a real page change toasts again");
    }

    #[test]
    fn toast_centers_over_the_zoom_slider() {
        let screen_center = 960.0;
        // The bottom bar's middle-third slider: the toast tracks its center.
        let slider = egui::Rect::from_min_max(egui::pos2(660.0, 800.0), egui::pos2(760.0, 812.0));
        assert_eq!(
            toast_offset(Some(slider), screen_center),
            660.0 + 50.0 - screen_center
        );
        // Before the first bottom-bar frame the toast stays screen-centered.
        assert_eq!(toast_offset(None, screen_center), 0.0);
    }

    #[test]
    fn focus_off_the_registered_edit_fields_is_transient() {
        let field = egui::Id::new("search_field");
        let editables = [(field, egui::Rect::ZERO)];
        assert_eq!(transient_focus(Some(field), &editables), None);
        let button = egui::Id::new("chrome_button");
        assert_eq!(transient_focus(Some(button), &editables), Some(button));
        assert_eq!(transient_focus(None, &editables), None);
    }

    #[test]
    fn recolor_keys_share_only_matching_page_colors() {
        let dark = Color::from([0x16, 0x18, 0x1D, 0xFF]);
        let light = Color::from([0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(recolor_key(dark, dark), recolor_key(dark, dark));
        assert_ne!(recolor_key(dark, dark), recolor_key(light, dark));
        assert_ne!(recolor_key(dark, dark), recolor_key(dark, light));
    }

    #[test]
    fn unfocused_windows_schedule_no_timed_repaints() {
        const CADENCE: Duration = Duration::from_millis(50);
        let toast_wait = Some(Duration::from_millis(120));
        // Everything outstanding, focused: the earliest wake wins.
        assert_eq!(
            should_schedule_repaint(true, true, None, true, toast_wait),
            Some(CADENCE)
        );
        // No pending renders: a due retry wakes exactly when it comes due.
        assert_eq!(
            should_schedule_repaint(true, false, Some(Duration::from_millis(250)), false, None),
            Some(Duration::from_millis(250))
        );
        // Nothing outstanding: no wake at all.
        assert_eq!(
            should_schedule_repaint(true, false, None, false, None),
            None
        );
        // Unfocused: silent even with everything outstanding — completion
        // results surface on the refocus frame instead.
        assert_eq!(
            should_schedule_repaint(
                false,
                true,
                Some(Duration::from_millis(250)),
                true,
                toast_wait
            ),
            None
        );
    }

    #[test]
    fn toc_rows_hover_with_the_hand_cursor_not_text() {
        let row = TocRow {
            title: "Chapter".into(),
            page: 0,
            dest_top: Some(0.0),
            depth: 0,
        };
        let ctx = egui::Context::default();
        let row_rect = std::cell::Cell::new(egui::Rect::NOTHING);
        // Frame one records where the row landed; frame two hovers it,
        // since cursor icons follow the previous frame's widget rects.
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = toc_row_ui(ui, &row, false, egui::Color32::GRAY);
                row_rect.set(response.rect);
            });
        });
        let mut input = egui::RawInput::default();
        input
            .events
            .push(egui::Event::PointerMoved(row_rect.get().center()));
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                toc_row_ui(ui, &row, false, egui::Color32::GRAY);
            });
        });
        assert_eq!(
            output.platform_output.cursor_icon,
            egui::CursorIcon::PointingHand,
            "the row's hand cursor must survive the title label"
        );
    }

    #[test]
    fn should_release_focus_decides_by_click_and_scroll_targets() {
        let field_id = egui::Id::new("field");
        let field = (
            field_id,
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(60.0, 20.0)),
        );
        let editables = [field];
        let inside = egui::pos2(30.0, 10.0);
        let outside = egui::pos2(300.0, 300.0);
        let with_focus = Some(field_id);

        // Nothing to release.
        assert!(!should_release_focus(
            Some(outside),
            None,
            None,
            &editables,
            false
        ));
        // A same-frame focus request wins over the blur.
        assert!(!should_release_focus(
            Some(outside),
            None,
            with_focus,
            &editables,
            true
        ));
        // Clicking inside the field never blurs it.
        assert!(!should_release_focus(
            Some(inside),
            None,
            with_focus,
            &editables,
            false
        ));
        // Clicks and scrolls outside release it.
        assert!(should_release_focus(
            Some(outside),
            None,
            with_focus,
            &editables,
            false
        ));
        assert!(should_release_focus(
            None,
            Some(outside),
            with_focus,
            &editables,
            false
        ));
        // No interaction, no release.
        assert!(!should_release_focus(
            None, None, with_focus, &editables, false
        ));
    }
}
