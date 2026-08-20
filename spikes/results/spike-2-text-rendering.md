# Spike 2 — TUI text rendering closure

Date: 2026-08-20  
Slice: 01-v0.1/09-candi-tui  
Implementation: `crates/candi-tui` (ratatui 0.30.2 + crossterm 0.29.0)

## Goal

Validate that extracted PDF text renders readably in the TUI with minimal
heuristics (word-wrap to terminal width, one page window in memory). Close Spike
2 from the v0.1 workflow.

## Corpus

| Source | Available locally? | Notes |
|---|---|---|
| `bench/corpus-local.toml` → frankl | **No** | File not present in this environment; no book PDF extracted or committed. |
| TestBackend fixtures | Yes | Integration tests in `crates/candi-tui/tests/ui.rs` |

## What was validated

1. **Single-column prose** — FakeDoc pages with long single paragraphs wrap to
   terminal width without panic; scroll bounds clamp via core `ViewState`.
2. **Page window (RSS)** — `App` holds `page_text` for the current page only;
   navigation reloads one page at a time.
3. **Resize** — `Event::Resize` recomputes wrap width and reclamps scroll; covered
   by `resize_rewraps_without_panic` TestBackend test.
4. **Error paths** — `page_text` failures surface in a dedicated error screen
   (`Error` Display); no silent blank UI.
5. **SSH / dumb terminal** — `run()` returns an error when `TERM=dumb` before
   entering raw mode; event loop uses `event::poll` with timeout (no busy spin).

## Known limitations (from Spike 1, unchanged)

These are **backend extraction** issues, not introduced by the TUI wrap layer:

- **Multi-column layouts** (e.g. attention paper): reading order may not match
  visual columns; the TUI displays extracted text in extraction order.
- **Footnotes / sidebars**: no reordering; footnote text may appear inline or
  out of visual order depending on the backend.
- **No pixel layout**: v0.1 TUI is text-only; equations, diagrams, and scanned
  pages without a text layer show the explicit `NoTextLayer` / backend error.

The TUI does not attempt to fix column or footnote ordering — documenting here
rather than silently misrepresenting layout.

## Pass / fail

| Criterion | Result |
|---|---|
| Single-column text wraps without crashing | **PASS** (TestBackend + unit wrap tests) |
| Limitations explicit | **PASS** (this doc + Spike 1 cross-ref) |
| frankl manual spot-check | **SKIPPED** — local corpus manifest absent |

## Recommendation

Ship v0.1 TUI text mode as implemented. A future slice may add optional
positional reordering once `page_positions` is consumed by frontends; that is
out of scope for 01/09.
