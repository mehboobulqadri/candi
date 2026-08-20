# Slice 01/09 — candi-tui

## Goal

The keyboard-first TUI: page view, status bar, search prompt + results, error
screen; resize handling; SSH-safe; `TestBackend`-tested. Also closes Spike 2.

## Prep / thinking

- ratatui + crossterm (pin versions — API churn is a known risk). Layout per
  architecture.md §Frontends: page view (paragraph wrapping), status bar (file,
  page x/y, search state), search prompt overlay, error screen.
- **Keybindings exactly per project.md §6** — no additions in v0.1.
- **Resize:** crossterm events → re-render. No display/GPU dependency — pure
  terminal, works over SSH (project.md 4.1 NFR). TERM=dumb / unsupported terminal →
  a minimal fallback (error screen, never a hang).
- **Spike 2 closure** (workflow Phase 1 leftover — do it here, not later): validate
  structured-text rendering on a single-column novel (frankl, local corpus book per
  the gitignored manifest) — must render
  cleanly with minimal heuristics; document the known multi-column/footnote
  limitations in spikes/results (limitations documented, not silently wrong — the
  spike's pass/fail).
- **UI architecture:** an app state machine (reading / searching / error) driving
  core APIs; core stays frontend-free (standing rule 4). Navigation/search logic is
  already tested in core — the TUI tests cover wiring, not re-testing core.
- **Tests:** ratatui `TestBackend` render assertions + injected key events — no
  PTY required.

## Files

- `crates/candi-tui/Cargo.toml` (ratatui, crossterm, candi-core)
- `crates/candi-tui/src/{main.rs, app.rs, ui.rs, keymap.rs, search.rs}`
- `crates/candi-tui/tests/ui.rs` (TestBackend)
- `spikes/results/spike-2-text-rendering.md` (closure doc)

## Implementation tasks

1. App skeleton: crossterm event loop, state machine, render loop.
2. Page view + status bar; scroll/page keys wired to core navigation.
3. Search prompt overlay + results; n/N wired to `SearchSession`.
4. Error screen mapping `Error` kinds to explicit messages (FR-010).
5. Resize handling.
6. Spike 2 validation run on the novel; write the closure doc.
7. `TestBackend` tests: layout renders, key events drive state.
8. Run the drill.

## Verification

- `TestBackend` tests green; manual run over SSH; error screens verified against
  encrypted/image-only/malformed fixtures.
- Drill stage 3: render-loop allocations (ratatui double-buffer — no per-frame
  allocation churn), no busy loops; stage 4: line-by-line.

## Commit message

```
feat(candi-tui): add keyboard-first TUI reader
```

## PR notes

- Merge target: `dev`.
- Reviewer: keybindings exactly per project.md §6; core gained no TUI logic;
  resize handled; every error path is explicit (no silent empty screen); the Spike
  2 closure doc exists with documented limitations.

## Risks

- ratatui API churn — pinned versions; an upgrade is its own slice later.
- Terminal quirks (SSH, TERM=dumb) — the fallback decision from prep must be
  implemented, not deferred.