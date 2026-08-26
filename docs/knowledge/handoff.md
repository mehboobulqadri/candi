# Handoff — next agent starts here

## State (2026-08-26)

Branch `slice/02-01-gui-reader`, tip `8ee2110` (reverted a broken
central-embedded sidebar experiment `2231aa8` — keep the sidebar as
SidePanels; see log), pushed, tree clean except the
user's reference images (`image.png`, `image copy.png` at repo root — the
six-state design reference; do NOT commit or delete them).

The GUI has been rebuilt against the user's six-state reference (root
`image.png`): always-visible icon rail (own SidePanel, 52px) + slide-out
section panel (Contents/Bookmarks/Search), hamburger + accent "Candi"
wordmark top bar (tagline removed by user request), page-nav pill, zoom
stepper deduped into the bottom bar (theme picker | − % + slider | view-mode
toggles), Inter typography (embedded OFL TTFs), Lucide SVG icons everywhere
(no font glyphs in chrome — fallbacks rendered tofu and mirrored ‹›), dark
theme is the startup default, styled empty state, real-content fixture
(arXiv 1706.03762) for captures.

Gates green: fmt, clippy -D warnings, workspace tests
(`PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` = DIRECTORY). Capture
evidence pipeline: `scripts/shot.sh` matches the spawned window by PID now —
older class-based matching grabbed other windows/wallpapers and produced
fake "broken UI" evidence more than once.

## Known-true facts about egui 0.30 (learned the hard way)

- Horizontal uis clamp children to one interact row (~18 px). Panels do not.
  Rail/section content lives in SidePanels for this reason; do not nest
  full-height content back inside `ui.horizontal`.
- `Layout::bottom_up` allocates from the cursor, not the rect bottom. Pin
  things by consuming remaining space or `ui.put` at exact rects.
- `add_sized` centers content — use `allocate_ui_with_layout` + halign for
  left/right alignment.
- `SidePanel::exact_width` beats `default_width` (state caching made widths
  stick from frame 1).
- RTL parents reverse child order — wrap steppers/pills in explicit
  `left_to_right`.
- `Slider` width is `spacing.slider_width`; fixed bars are `exact_height`.
- Menus with custom widgets: `popup_below_widget` + `toggle_popup`.
- `pkill -f '<path regex>'` kills your own shell; use `pkill -x candi`.

## Next (user stop-gates — explicit authorization required)

1. Finish the visual iteration loop: recapture s8 light/dark once the user
   closes their wallpaper-picker overlay (it floated over the last two
   captures), review, fix residuals.
2. User dogfoods → tag `v0.1` → PR `slice/02-01-gui-reader` → `dev` → `main`.
   Release flow per user: slices merge to dev; every 0.x tag lands on main.

## Residual polish (judgment calls, not defects)

- ~1px separator + shadow line between section panel and canvas — reads as
  an intentional divider; user has seen it.
- Theme picker paints chevron left of the name (reference wants right).
- Page pill sits after the wordmark, not centered over the canvas.
- View modes are 3 honest toggles (free/fit-width/fit-page), not the
  reference's 4 (dual/mobile are unbuilt features — do not fake).

## Roadmap (user-set)

- v0.2 performance/lean pass. v0.3 mobile/other-platform ports. v0.4 docs +
  architecture write-up.
- Deferred UI features (user said NOT now): search top-overlay redesign,
  visual Appearance panel (accent swatches/font-size), custom window
  decorations (− □ ✕), dual-page/mobile view modes.

## Environment notes

- The user works on this machine while agents run: expect window interference;
  captures must be PID-matched and re-checked. User can provide fullscreen
  views on request.
- Disk cleanup was deferred by the user: `~/.local/share/opencode/opencode.db`
 is 26G (session history), `target/debug` ~23G, `spikes/pdf-backend/target` 2G.
