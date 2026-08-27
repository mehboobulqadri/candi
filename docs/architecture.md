---
title: Architecture
nav_order: 5
---

# Candi — Architecture

Candi is a minimal, fast, cross-platform document reader built as one shared native Rust
core with two frontends: a keyboard-first terminal UI (`candi-tui`, v0.1) and a graphical
UI with rendered PDF pages (`candi` in candi-gui, v0.1). All document-independent logic —
navigation, search, reading position, configuration — lives in candi-core and is exposed
through one API both frontends share.
PDF access sits behind a single document-backend trait in candi-pdf with two
runtime-switchable engines: MuPDF (default) and PDFium. Both ship in v1; the user
evaluates them in real use, and the final single engine is chosen later. The core never
knows whether it is driven by the TUI or the GUI, and no frontend duplicates document
logic.

## Workspace & dependencies

```text
  candi-tui (bin)              candi-gui (bin `candi`)
       │                              │
       │         candi-cli (lib) ◄────┘   open/resume/sidecar helpers
       │              │
       └──────┬───────┘
              │
          candi-core  (nav, search, position, config)
              │
          candi-theme  (semantic tokens; UI-independent)
              │
          candi-pdf  (Document/Backend trait + engines)
           ├── mupdf-backend    feature `mupdf-backend`,  default — AGPL-3.0
           └── pdfium-backend   feature `pdfium-backend`, default — BSD-3-Clause

Arrows = depends on. TUI/GUI → core (+ candi-cli for GUI open path); core → candi-pdf
(trait only); bindings/python (v0.5) wraps candi-core.
```

## Design decisions

Every decision below was validated by Spike 1 (spikes/results/spike-1-pdf-backend.md)
and its merciless review; do not re-open without a spike that contradicts the data.

| Decision | Choice | Rationale | Alternatives considered | Status |
|---|---|---|---|---|
| PDF engine | Dual MuPDF + PDFium, runtime-switchable (`--backend mupdf\|pdfium`, default mupdf; config.toml key later) | Both pass spike 1 (reading order, fidelity, licensing); user evaluates both in real use, picks the final one later | Single engine; Poppler (rejected: slowest, no structured API, GPL, 15 MB glib baseline) | decided |
| License & escape hatch | AGPL-3.0 for v1; PDFium impl behind its own cargo feature, so `--no-default-features --features pdfium-backend` links zero AGPL code | MuPDF statically linked → the app is AGPL (accepted); the feature de-risks a future permissive/commercial re-license without code changes | Artifex commercial license (cost); pdfium-only from day one (loses MuPDF quality) | decided |
| Backend impl location | Feature-gated modules inside candi-pdf (`mupdf-backend`, `pdfium-backend`, both default-on) | One crate, one trait, one test suite; features provide the license escape hatch | Separate impl crates (rejected: shared trait/error types across crates, duplicated parity tests, feature spread) | decided |
| Trait error model | Enum by kind: `NotFound`, `PermissionDenied`, `Encrypted`, `WrongPassword`, `NoTextLayer`, `Malformed`, `Unsupported`, `Other` | Kinds are stable and UI-matchable; MuPDF exposes error codes — use them, never parse messages | String matching (rejected: brittle; reviewed bug class) | decided |
| Cached, lazy access | `page_count()` cached & infallible after open; `page_text(page)` extracts one page on demand, never the whole document | Navigation needs O(1) count; v0.1 TUI/search load a page window — `reader_peak` ~44 MB on silberschatz; a sequential all-pages sweep still hits MuPDF's ~295 MB `full_pass_peak` (256 MiB decode store), which is not the reader path | Whole-doc extraction (rejected) | decided |
| Positions API & semantics | `Result<Option<PagePositions>>` — real errors propagate, `Ok(None)` only when a backend has no positional API; block/line/word definitions pinned once at the trait level | The spike's `Option<Stats>` conflation was a reviewed bug; backends count differently on the same page (15 vs 67 blocks) | `Option<Stats>`; per-backend semantics (rejected) | decided |
| Engine ownership | MuPDF `Document` owns its context; the PDFium engine is Arc-shared, created by the backend factory, held outside per-document objects | The spike's `Box::leak` was accepted only for a once-per-process probe; production leaks nothing | `Box::leak` (rejected) | decided |
| Search (FR-008) | Core-level abstraction over lazy per-page text: page-at-a-time scan, result list, next/prev (v0.1) | Document-independent; the backend only supplies text | Backend-level search API (rejected: duplicates core logic) | decided |
| Workspace layout | crates/candi-core, candi-pdf, candi-theme, candi-tui, candi-gui, candi-cli (lib); bindings/python (v0.5) | Per project.md; user binaries `candi-tui` + `candi`; core never gains frontend logic | — | decided |
| Themes, config & state | Semantic tokens + TOML + safe fallback with reported error; XDG config (`~/.config/candi/{config.toml,themes/}`); versioned sidecar `book.pdf.candi.toml`, atomic writes | UI-independent theming; platform conventions; the source PDF is never modified | Widget-specific keys (rejected) | decided |
| Security posture | Untrusted PDFs: no JS execution, bounded resources, explicit errors; MuPDF's silent-open-on-truncated is detected and errored | Backend reality from the spike (MuPDF opens truncated files as 0-page docs) | — | decided |
| Quality drill | 4 stages per task: organized write → static analysis → performance review → merciless line-by-line review | User mandate; non-negotiable | — | decided |
| Benchmarks | A gate per phase; both backends; best-of-2; process-level RSS methodology from the fixed probe | workflow.md standing rule 2; numbers defined before implementation | — | decided |
| v0.1 scope | `candi-tui` + `candi` (GUI text), scroll, first/last page, search + n/N, position persistence, default keybindings, graceful errors, SSH (TUI) | project.md 4.1; GUI text pulled into v0.1; pixel PDF pages deferred | original wider draft (deferred to v0.2+) | decided |

## candi-pdf

### Trait

```rust
pub enum Error {
    NotFound(String), PermissionDenied(String), Encrypted(String),
    WrongPassword(String), NoTextLayer, Malformed(String),
    Unsupported(String), Other(String),
}

pub struct PagePositions { pub blocks: Vec<Block> }
pub struct Block { pub lines: Vec<Line> }   // contiguous text region ≈ paragraph
pub struct Line  { pub words: Vec<Word> }   // words sharing one baseline
pub struct Word  { pub text: String, pub x: f32, pub y: f32, pub font_size: f32 }
pub struct PageImage { pub width: u32, pub height: u32, pub rgba: Vec<u8> }
pub struct TocItem { pub title: String, pub page: usize, pub children: Vec<TocItem> }

pub trait Document: Send + Sync {
    fn page_count(&self) -> usize;                               // cached at open; infallible
    fn page_text(&self, page: usize) -> Result<String, Error>;   // lazy, one page
    fn page_positions(&self, page: usize) -> Result<Option<PagePositions>, Error>;
    fn page_size(&self, page: usize) -> Result<(f32, f32), Error>; // PDF points (w, h)
    fn render_page(&self, page: usize, scale: f32) -> Result<PageImage, Error>;
    fn outline(&self) -> Result<Vec<TocItem>, Error>;            // empty when absent
    fn search_page(&self, page: usize, needle: &str)             // case-insensitive match
        -> Result<Vec<[f32; 4]>, Error>;                         // rects in points, top-left origin
}

pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;
    fn open(&self, path: &str, password: Option<&str>) -> Result<Box<dyn Document>, Error>;
}
```

### Contracts

- **Error by kind.** `Encrypted` vs `WrongPassword` decided by whether a password was
  supplied to `open()` (MuPDF can also distinguish via `authenticate`); backends map
  native error codes (mupdf `fz_error_code`, pdfium `FPDF_ERR_*`) to kinds, unknown →
  `Other`. Messages are for humans, never matched on.
- **Lazy loading.** `open()` parses, validates, caches `page_count`, loads no text;
  `page_text(p)` extracts exactly page `p`; empty page → `Ok("")` (cover pages are
  legitimately empty). `NoTextLayer` is document-level, detected at open by sampling the
  first pages — all empty → image-only/scanned → OCR-not-supported message (FR-003).
- **Positions semantics (pinned once).** block = contiguous text region (≈ paragraph);
  line = maximal run of words sharing a baseline; word = whitespace-free run with origin
  + font size. Backends normalize to these; PDFium approximates blocks/lines from text
  objects/segments, so counts differ from MuPDF (67 vs 15 blocks on one page) while
  semantics hold; feeds future reading-order reconstruction (FR-004).
- **Engine ownership.** MuPDF's `Document` owns its `fz_context` (open → drop). PDFium's
  engine is `Arc<Pdfium>`, created once by the backend factory (dlopens `libpdfium`),
  held outside documents; each `Document` holds the `Arc` plus its `FPDF_DOCUMENT`.
  Nothing leaks.

### Backend features & runtime selection

```toml
[features]
default = ["mupdf-backend", "pdfium-backend"]
mupdf-backend  = ["dep:mupdf"]
pdfium-backend = ["dep:pdfium-render"]
```

Both default-on for v1. Runtime selection:

```rust
pub enum BackendKind { Mupdf, Pdfium }

pub fn available() -> Vec<&'static str>;          // compiled-in backends
pub fn open(kind: BackendKind, path: &str, password: Option<&str>)
    -> Result<Box<dyn Document>, Error>;          // default: Mupdf
```

The GUI passes `--backend mupdf|pdfium` (default mupdf; config key later); unknown
names are `Unsupported`. The TUI uses the default backend factory.

### License implications

| Build | Features | Linked engines | Engine licenses | Candi's own code | Notes |
|---|---|---|---|---|---|
| Default (v1) | both backends | MuPDF (static) + PDFium (dlopen) | AGPL-3.0 + BSD-3-Clause | AGPL-3.0 | standard binary; runtime switch |
| Permissive escape hatch | `--no-default-features --features pdfium-backend` | PDFium only | BSD-3-Clause (wrapper MIT/Apache-2.0) | AGPL-3.0 in v1 | zero AGPL third-party code compiled or linked; keeps a future permissive/commercial re-license build-ready without code changes |

Candi's own crates are AGPL-3.0 in v1 (accepted decision); the escape hatch guarantees
an AGPL-free *third-party* build — a future fully-permissive or commercially-licensed
distribution is a config change, not a rework.

## candi-core

Document-independent logic; the only frontend-agnostic consumer of the `Document` trait.

- **Navigation model.** View state = `(page, scroll_offset)`: j/k scroll within the page;
  h/l navigate pages; g/G first/last; bounds against cached `page_count()`.
- **Reading position & session (sidecars).** `book.pdf.candi.toml` next to the PDF, never
  the PDF itself. Versioned from first release; atomic writes (temp file + rename). Two
  records share the file: the TUI's position (schema v1) and the GUI's full session
  (schema v3 — reading position with scroll fraction, zoom, theme, plus bookmarks; v3
  adds optional bookmark titles, v2 files parse as title-less bookmarks and v1 is
  migrated on load).

  ```toml
  schema_version = 3
  updated_at = "2026-08-20T12:00:00Z"
  [reading]
  page = 42
  scroll_frac = 0.31
  zoom = 150
  theme = "Dark"
  [[bookmarks]]
  page = 57
  created_at = "2026-08-20T12:05:00Z"
  title = "good chart"    # optional in v3
  ```

  Written on quit and on page change; a missing sidecar is fine (fresh start); future
  format work grows from here per Spike 4.
- **Search abstraction.** `SearchSession { query, results: Vec<(page, offset)>, cursor }`
  over lazy `page_text`: pages extracted and scanned one at a time on demand — first
  result cheap, full scan never materializes the document. n/N moves the cursor; the TUI
  renders the result's page; the result list holds page numbers + offsets only.
- **Config.** XDG lookup (`~/.config/candi/config.toml`); v0.1 reads minimal keys
  (backend selection later); CLI flags take precedence over file keys.

## Frontends

- **TUI (candi-tui, v0.1)** — ratatui + crossterm, keyboard-first. Layout: page view,
  status bar (file, page x/y, search state), search prompt, error screen; resize via
  crossterm; no display/GPU dependency — works over SSH. Ligature-normalized text,
  centered column, mouse wheel and j/k continuous page scroll. Keybindings fixed per
  project.md §6.
- **GUI (candi-gui, v0.1)** — egui/eframe + rfd file dialog. Binary name `candi`.
  No args opens a file picker; path arg opens that PDF. Rendered PDF pages (RGBA via
  `render_page`, recolored to the theme) on a paged canvas: continuous scroll, single-page,
  or dual-page spreads, with fit-width/fit-page flows, anchor-preserving zoom and pinch.
  Shell: icon rail; sidebars for contents (outline), search with in-page highlight rects,
  appearance (themes + editor), bookmarks and keybinds editing; recents; drag-and-drop
  open. Session persistence (page, zoom, theme, bookmarks) via the schema-v3 sidecar
  through candi-cli helpers.

## candi-theme

YAML themes, not widget keys, so one theme works across TUI and GUI. A theme pins
`page_bg`, `page_fg`, `ui_bg`, `panel_bg`, `ui_fg`, `accent`, and `selection`:

```yaml
name: Dark
page_bg: "#16181D"
page_fg: "#E6E6E6"
ui_bg: "#1D2026"
panel_bg: "#26292F"
ui_fg: "#CCCCCC"
accent: "#4C8DF6"
selection: "#4C8DF666"
```

v0.1 ships five built-in themes (`Light`, `Sepia`, `Warm Dark`, `Dark`, `True Dark`),
embedded as YAML and selected by name. Users add their own as
`~/.config/candi/themes/<name>.yaml` (file stem must match the embedded `name`), or craft
one in the GUI's theme editor and save it there. Two rendering passes make a theme carry
over pixels: [`recolor`](crates/candi-theme/src/recolor.rs) maps rendered page bitmaps
onto the theme's page colors while protecting saturated figure pixels, and
[`retint`](crates/candi-theme/src/retint.rs) blends a small, luma-gated fraction of the
accent into the chrome backgrounds.

Validation: the schema is strict — unknown fields are a fatal `Schema` error
(`deny_unknown_fields`), never silently ignored — while each missing color token
defaults from the embedded Light palette, so a theme may override as little as one
token. A theme file that fails to parse is reported and skipped where it loads; in
the GUI's theme editor every edit reparses live and surfaces the error inline while
the last-good theme stays active. User theme YAMLs carry no version field today:
`deny_unknown_fields` would reject one, and the nested/flat parser flattening is
the compatibility layer.

## Security

PDFs are untrusted, attacker-controlled input. Threat model: RCE on the host,
confidentiality of the user's files, availability (hang / exhaustion). Attack surface:
the parser and extraction code of the two C engines.

- **No code execution.** MuPDF has no JavaScript engine — embedded JS never runs.
  PDFium's JS requires a V8-enabled build; the prebuilt library and Rust wrapper used
  here are built without JS. Neither backend runs external programs. v0.1 opens no
  embedded files and follows no links.
- **Resource exhaustion.** Decompression bombs and huge pages are bounded by lazy
  per-page extraction (never whole-document), no unbounded caches, a peak-RSS benchmark
  gate. Error/malformed fixtures run in CI; the benchmark harness is timeout-wrapped.
- **Malformed input — MuPDF's silent open.** The spike showed MuPDF opens a truncated
  PDF "successfully" as a 0-page document, failing confusingly only later. Production
  `open()` must detect this: after open, `page_count() == 0` → `Error::Malformed`
  ("truncated or empty document"). PDFium rejects at open; parity tests pin both.
- **Encrypted / image-only.** `Encrypted` / `WrongPassword` with a clear message;
  image-only → `NoTextLayer` with the OCR-not-supported message (project.md 4.6) —
  never a silent empty screen.
- **Graceful errors.** Every failure path maps to an `Error` kind surfaced explicitly;
  no silent failures (§ Cross-cutting). Hardening is implementation Phase 1; parser
  fuzzing is future work (post-v0.1).

## Cross-cutting

- **Error handling philosophy.** Errors are typed data, never strings, never swallowed:
  the UI maps kinds to explicit messages, the CLI exits nonzero, every `Err` path is
  tested. The only sanctioned fallback is the theme fallback (§ candi-theme), and it
  reports.
- **Logging.** Minimal. Diagnostics to stderr behind `--verbose`; no log files in v0.1.
- **Performance budget.** Defined before implementation (workflow.md rule); best-of-2,
  process-level RSS measurement (baseline VmRSS, VmHWM peak, delta).

| Metric | Target (best-of-2) | Spike reference |
|---|---|---|
| Startup to first page | < 300 ms | open 1–116 ms + process init |
| Open (file → page_count) | ≤ 150 ms on all corpus docs | 1–116 ms |
| Page text extraction | < 20 ms/page mean on corpus | ~2–4 ms/page |
| Search first result | < 300 ms on any corpus doc | extraction-bound |
| Search next/prev | < 50 ms | in-memory cursor |
| Peak RSS (reader_peak) | < 200 MB on corpus | MuPDF silberschatz ~44 MB (open + first page + search/nav page window); `full_pass_peak` ~295 MB on full page sweep is MuPDF `FZ_STORE_DEFAULT` (256 MiB) — recorded, not gated; PDFium silberschatz reader ~58 MB, full_pass ~92 MB |

## Open questions / future

Remaining open items (tracked in spikes.md) block nothing in v0.1:

- Spike 2 — text-first rendering quality (closed: TUI readability pass); Spike 3 —
  GUI framework (resolved: egui/eframe — immediate-mode kept the shell thin, single
  dependency for windowing + GPU drawing); Spike 4 — sidecar format & concurrent
  access (v0.2).
- Spike 5 — terminal images (v0.9); Spike 6 — Python bindings (v0.5); Spike 7 — packaging (v0.5).
