# Slice 01/06 — candi-core-navigation

## Goal

The navigation model in candi-core: view state (page + scroll offset), first/last,
next/prev, bounded against the cached `page_count()` — frontend-agnostic and
unit-tested.

## Prep / thinking

- View state = `(page, scroll_offset)` — nothing else (architecture.md §candi-core).
- **Page indexing:** decide and pin 0-based in core (the TUI displays x/y 1-based);
  document the choice.
- **Scroll:** j/k move `scroll_offset` by a delta; clamping needs a page-height
  concept — decide whether core knows line counts (it must for clamping) or the TUI
  passes limits in. Keep the interface minimal; nothing TUI-specific leaks into
  core (standing rule 4).
- **Page navigation:** h/l next/prev, g/G first/last — bounds against the cached
  `page_count()`; out-of-range → **clamp** (decide; clamping is friendlier and the
  TUI never shows errors for normal keys).
- **Edge cases:** empty document (`page_count() == 0`) must not panic; single-page
  docs; scroll beyond the last line clamps.
- Core must be testable without a UI (REQUIREMENTS.md NFR) — no ratatui/crossterm
  types anywhere in this crate.

## Files

- `crates/candi-core/Cargo.toml` (dep: candi-pdf)
- `crates/candi-core/src/lib.rs`
- `crates/candi-core/src/navigation.rs`
- `crates/candi-core/tests/navigation.rs`

## Implementation tasks

1. `ViewState { page, scroll_offset }` with typed accessors.
2. `next_page`/`prev_page`/`first_page`/`last_page`/`scroll_up`/`scroll_down` with
   clamping against `page_count` and page height.
3. Unit tests: every operation, bounds, clamping, empty doc, single-page doc.
4. Run the drill.

## Verification

- Unit tests cover every operation + edges.
- Drill stage 2: fmt / clippy / deslop. Stage 3: no allocations per keypress
  beyond the state copy; no module-level mutable state. Stage 4: line-by-line.
- CI: candi-core tests in the matrix.

## Commit message

```
feat(candi-core): add navigation model
```

## PR notes

- Merge target: `dev`.
- Reviewer: clamping semantics (no panics on empty/out-of-range), the 0-based pin
  documented, no frontend types in core.

## Risks

- Page-height semantics could leak TUI concerns into core — the scroll-limit
  interface must stay minimal (decide in prep, before writing).