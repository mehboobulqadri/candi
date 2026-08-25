# Handoff — next agent starts here

## State (2026-08-25)

Branch `slice/02-01-gui-reader`, tip `5dbf737`, pushed. v0.1 GUI reader is
feature-complete and visually iterated: Lucide SVG chrome icons, 46/42 px
flat top/bottom bars, full-height icon rail (accent active + indicator, gear
pinned bottom), vertical sidebar with right-aligned TOC page numbers, canvas
GAP 16 / MARGIN 20, five themes, live syntax-highlighted YAML theme editor,
inline page jump, bookmarks/search panels, focus/empty/error states,
drag-and-drop. Committed 10-shot matrix in `docs/design/screenshots/`
recaptured on the final UI. Gates green (fmt, clippy -D warnings, workspace
tests with `PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` — DIRECTORY, not
file).

## Next (user stop-gates — explicit authorization required)

1. User dogfoods the binary.
2. Tag `v0.1`.
3. PR `slice/02-01-gui-reader` → `dev`; then `dev` → `main`.
   Release flow per user: every 0.x version lands on `main`.

## Roadmap (user-set)

- v0.2: performance/lean-out pass (memory, render pipeline).
- v0.3: mobile + other platform ports.
- v0.4: docs + architecture write-up, detailed implementation notes.

## Hard-won environment facts

- `pkill -f '[t]arget/release/candi'` SELF-MATCHES the driving shell's
  cmdline and kills it (the "stuck builder" plague). Use `pkill -x candi`.
- egui 0.30: horizontal uis clamp children to one interact row (~18 px) —
  set explicit height; `bottom_up` flows from the cursor, pin by consuming
  remaining space; `TopBottomPanel::exact_height`; `Slider` width via
  `spacing.slider_width`; menus with custom widgets via
  `popup_below_widget` + `toggle_popup`.
- Fixture: `ghost-outline.pdf` stands in for demo books (outline + blank
  pages); a missing PDF shows the error card — do not mistake it for a
  layout bug.
