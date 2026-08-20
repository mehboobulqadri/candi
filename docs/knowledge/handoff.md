# Handoff

State for the next agent. Written at close-out; read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Phase 01 reader on `dev`** — slices 01/01–01/11 plus post-release TUI/GUI work
  (PRs #4–#18). User binaries: `candi-tui` (TUI) and `candi` (GUI text). `candi-cli`
  is a shared lib (open/sidecar); no user-facing CLI reader.
- **PR #18 gui-text-linux** merged (`54d9f26`): egui GUI, integration tests in
  `crates/candi-gui/tests/cli.rs` (`CANDI_NO_GUI=1` test hook only).
- **PR #17 tui-readability** merged (`24dff95`): ligatures, centered column, scroll polish.
- **CI:** Linux-only rust-checks matrix; Windows job removed/deferred with PR #18.

## In progress

Nothing active.

## Next

**User-authorized release gates only** — no further engineering until user explicitly
authorizes:

1. **Dogfood gate** — daily-use evidence before tag (not met).
2. **Tag `v0.1`** — after dogfood on `dev`.
3. **`dev` → `main`** — after tag.

Mia must not proceed on any of the above without explicit user OK.

## Open questions

- **Unsupported PDF-feature row** in hardening table — deferred in 01/04.
- **MuPDF store cap / PDFium-for-full-doc-search** — deferred.
- **GUI pixel page rendering** — later phase; Spike 3 framework choice still open.

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **Release gates** | Dogfood, tag, and `main` merge are user stop-gates — not engineering work. |
| **`CANDI_NO_GUI=1`** | Test/CI hook for `candi` headless spawn tests — not a user feature. |
| **Bench corpus** | Real books in gitignored `bench/corpus-local.toml` — never commit. |
