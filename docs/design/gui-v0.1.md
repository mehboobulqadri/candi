# Candi GUI v0.1 — Design Guide

This document is the authoritative design specification for the Candi desktop
GUI (`candi`) at v0.1. Reference mockups:

- `docs/design/mockups/ideal-ui-dark.png` — target aesthetic
- `docs/design/mockups/ideal-ui-light.png` — alternate theme

## 1. Goal

A browser-PDF-reader-style desktop reader: open a PDF, scroll it continuously,
navigate and search it, with a polished, themeable interface. Polish is valued
over feature count.

Non-goals for v0.1:

- Annotations beyond bookmarks (no highlights, notes, or drawing)
- Dual-page view
- Changes to the TUI
- Performance tuning beyond the RSS cap (~200 MB, benchmark gate `reader_peak`)

## 2. Layout

### Top bar

Left to right:

- Hamburger menu: Open File (Ctrl+O), Save State
  (Ctrl+S), Theme ▸ (built-in themes + Edit theme YAML… Ctrl+E), About Candi.
  Open Recent ▸ is deferred — no implementation planned for v0.1.
- Centered document title.
- Page navigation: ‹ n/N › (previous page, current page indicator, next page).
- Collapsible search field on the far right.

### Left sidebar

Toggled with Ctrl+B. Width ~260 pt. Structure top to bottom:

- Header row: `DOCUMENT` label plus the filename.
- Navigation rows: Contents / Bookmarks / Search, each showing an item count.
- Section content below the active nav row:
  - **Contents**: TOC tree; clicking an entry jumps to that section's page.
  - **Bookmarks**: list of bookmarks (page number) with
    add/remove controls.
  - **Search**: search box plus results list; clicking a result jumps to it.

### Center canvas

Continuous vertical scroll of rendered page bitmaps, centered horizontally with
~8 pt gaps between pages and a subtle border around each page. While a page is
rendering, its area shows placeholder fill in the page background color.

**Theme editor mode** replaces the canvas entirely: a monospace YAML editor for
the active theme, with an error banner shown above it when parsing fails (on
parse error the last-good theme stays active) and a hint line describing the
schema. Esc returns to the page view.

### Bottom bar

- Theme dropdown: built-in themes + Edit… entry.
- Zoom controls: − / percentage display / +. Clicking the percentage resets to
  fit-width.
- Fit-width button.

## 3. Keyboard

| Key | Action |
|---|---|
| Ctrl+O | Open file |
| Ctrl+S | Save state |
| Ctrl+F | Focus search |
| Ctrl+B | Toggle sidebar |
| Ctrl+E | Edit theme YAML |
| `+` / `-` | Zoom in / out |
| `0` | Fit-width zoom |
| Arrows, PgUp/PgDn | Navigate document |
| T | Cycle built-in themes |
| Esc | Close overlays / exit theme editor |
| q | Quit |

## 4. Theme system

Themes are user-editable YAML files, opened in-center via Theme ▸ Edit theme
YAML… or the bottom bar's Edit…. The schema is strict (`deny_unknown_fields`),
colors are hex `#RRGGBB` or `#RRGGBBAA`, and every field except `name` has a
default:

```yaml
name: Sepia
page_bg: "#F4ECD8"
page_fg: "#3B3228"
ui_bg: "#262019"
panel_bg: "#332B21"
ui_fg: "#D8CFC0"
accent: "#C89B3C"
selection: "#A85D3C66"
```

Built-in themes are embedded as YAML strings parsed through exactly the same
code path as user files: Light, Sepia, Warm Dark, Dark, True Dark.

UI tokens (`ui_bg`, `panel_bg`, `ui_fg`, `accent`, `selection`) drive
the egui style at runtime; `page_bg`/`page_fg` drive bitmap recoloring. A parse
error keeps the last-good theme active and raises the error banner.

Dependency note: theming uses `serde_yaml` 0.9. The crate is archived but
stable (dtolnay); this is accepted deliberately. Revisit only if it breaks.

## 5. Recolor algorithm — guarded luminance LUT

Inspired by zathura's fast path plus Dark Reader's neutrality thresholds. Pure
integer math, single u8 pass per pixel, no floats:

1. Luma: integer Rec.601, `l = (299r + 587g + 114b) >> 10`.
2. Saturation guard: if `max(r,g,b) − min(r,g,b) > 48`, leave the pixel
   untouched. This protects figures and images from being flattened.
3. Three 256-entry channel lookup tables map luma →
   `lerp(fg, bg, t)` where `t = clamp((v − p2) / (p95 − p2))`.
4. `p2` / `p95` come from a sampled 256-bin histogram (every 4th pixel). When
   the domain is already near-black-on-white (`p95 ≥ 235 && p2 ≤ 20`) the
   identity mapping applies.

The texture cache stores **original** (unrecolorized) bitmaps. Recoloring runs
when promoting a cached bitmap to a GPU texture, so switching themes re-runs
only the pass over cached originals (~ms per page) rather than re-rendering.

## 6. Rendering architecture

- Worker thread + mpsc channel; requests coalesced latest-per-page.
- Priorities: current page > adjacent pages > prefetch (±2).
- `ctx.request_repaint()` when a render completes so the UI picks it up.
- First page renders synchronously on open so something is visible immediately.
- Bitmap cache: LRU keyed `(page, scale_q)`, byte-budgeted at 192 MB,
  holding original RGBA bitmaps.
- Textures: `ColorImage::from_rgba_unmultiplied` + `ctx.load_texture` once per
  slot; `TextureHandle::set` on re-render. LINEAR filtering.
- Render scale = pdf_points × zoom × pixels_per_point. Zoom quantized to ~5%
  steps so the cache key space stays small.
- During re-render the stale texture is shown scaled to the new size (SumatraPDF
  pattern) instead of blanking.

## 7. Scroll & state model

Content height is precomputed from page aspect ratios × zoom, which makes the
visible page range a clip_rect intersection rather than a scan.

Sidecar state file schema v2:

```yaml
page: <int>
scroll_frac: <0..1>        # position within document, zoom-independent
zoom: "fit-width" | <percent>
theme: "<theme name>"
bookmarks:
  - page: <int>
    created_at: <timestamp>
```

v1 → v2 migration: keep `page`; everything else defaults. Known limitation:
the TUI does not touch sidecars, and the headless `CANDI_NO_GUI=1` mode reads
schema v1 only — it hard-fails with UnsupportedSchema on GUI-written v2 files
(pinned by `crates/candi-gui/tests/cli.rs`), so after a GUI session the
headless smoke-check exits 1 for that book.

## 8. Testing strategy

Pure logic is unit-tested egui-free: recolor pass, theme parsing, sidecar v2
read/write/migration, cache byte accounting, layout geometry.

kittest was evaluated as a UI test harness; its GPU requirement was checked
empirically. Fallback where unavailable: logic tests plus scripted live-session
screenshots via grim/hyprctl under `scripts/`.

The `CANDI_NO_GUI=1` test/CI contract is preserved.

## 9. CI/repo cleanup plan (later slice)

Deferred to a dedicated slice:

- Collapse the single-entry OS matrix in `ci.yml`.
- Extract a composite action for libpdfium setup shared by rust-checks and bench.
- Prune the vestigial `/_site` gitignore rule if unused.
