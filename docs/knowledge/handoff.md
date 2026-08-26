# Handoff — next agent starts here

## State (2026-08-26, late)

Branch `slice/02-01-gui-reader`, tip `49be1fc`, pushed, gates green (fmt,
clippy -D warnings, workspace tests with
`PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` = DIRECTORY).

The GUI is feature-complete against the user's six-state reference
(root `image.png`): icon rail (52px SidePanel, gear bottom) + slide-out
section panel, hamburger + accent "Candi" wordmark (no tagline), centered
page pill, Lucide SVG icons everywhere, Inter typography embedded, dark
default, four page flows (Continuous / Single / Dual spreads / Fit page),
top-bar search overlay (query + n/m count + prev/next + results panel),
in-page match highlighting (both MuPDF and PDFium implement rect search —
trait `Document::search_page`), named bookmarks (schema v3, v2 sidecars
still parse; inline rename via pen icon), Appearance panel (rail gear:
theme picker, 8 accent swatches, text-size slider driving
pixels_per_point), in-app window decorations (decorations off; custom
minimize/maximize/close + brand-zone drag; `ViewportCommand::Minimized`
exists, `Minimize` does not), drag-and-drop, focus/empty/error states.

## Install (done on this machine)

- `~/.local/bin/candi` → symlink to `target/release/candi` (stale
  `~/.cargo/bin/candi` removed — it shadowed the symlink once).
- Desktop entry `~/.local/share/applications/candi.desktop`
  (Icon=candi at ~/.local/share/icons/hicolor/128x128/apps/candi.png,
  MimeType=application/pdf, StartupWMClass=candi).
- NOTE: the symlink points into `target/release` — a `cargo clean`
  invalidates it until the next build.

## Capture discipline (do not regress)

`scripts/shot.sh` matches the spawned window BY PID. Older class-based
matching grabbed other windows/wallpapers and fabricated "broken UI"
evidence. Verify WHAT you captured; the user works on this machine
simultaneously — expect interference, ask for quiet moments for final
evidence. User's display runs a scale factor: logical vs physical sizes
differ in captures.

## egui-0.30 gotchas (all learned the hard way, all verified)

- Horizontal uis clamp children to one interact row (~18px). Panels don't.
  Full-height content (rails, panels) lives in SidePanels.
- `Layout::bottom_up` allocates from the cursor — pin by consuming
  remaining space or `ui.put` at exact rects.
- `add_sized` centers its content — `allocate_ui_with_layout` + halign for
  left/right alignment.
- `SidePanel::exact_width` (default_width caches frame-1 width).
- RTL parents reverse child order — wrap steppers/pills in explicit LTR.
- `Slider` width = `spacing.slider_width`; bars = `TopBottomPanel::exact_height`.
- Custom-widget menus: `popup_below_widget` + `toggle_popup`.
- CentralPanel/panel seams: set the frame `.fill(panel_bg)` +
  `.inner_margin(0)` or the clear color shows as a dark band.
- `pkill -f '<path regex>'` kills your own shell — `pkill -x candi`.
- `ViewportCommand::Minimized(bool)` exists; `Minimize` does not.

## Next (user stop-gates — explicit authorization required)

1. User dogfoods (now launchable as `candi` anywhere + app-list entry).
2. User names missing pieces from the six-state reference — remaining
   known gap: none functional; polish judgment calls (separator line,
   picker chevron side) documented in this file's earlier revisions.
3. Tag `v0.1` → PR `slice/02-01-gui-reader` → `dev` → `main`.
   Release flow: slices merge to dev; every 0.x tag lands on main.

## Deferred (user-set)

- Disk cleanup: `~/.local/share/opencode/opencode.db` = 26G (session
  history; VACUUM approach documented in log.md), `target/debug` ≈ 23G,
  `spikes/pdf-backend/target` = 2G.
- Search: top overlay shipped; in-page HIGHLIGHTING shipped; "highlight
  current vs other matches differently" not done.
- Possible polish: per-app persistence of flow choice; accent/YAML
  persistence across restarts (currently resets on reopen by design).
- Docs slice (v0.4 per roadmap): architecture.md §candi-pdf trait block is
  stale (missing page_size/render_page/outline/search_page).
