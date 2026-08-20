# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Slice 01/10 candi-cli merged to `dev`** (PR #15, https://github.com/mehboobulqadri/candi/pull/15): slice feat `de1ecce`, fix `5089cc6`, merge `9b86661`, mergedAt 2026-08-20T19:59:31Z. Binary `candi`; clap FILE + `--backend` (default mupdf); `Args::try_parse()` — all parse failures **exit 1** (usage, unknown flags). Open → sidecar load → resume via `ViewState::goto_page`; corrupt sidecar warns; `UnsupportedSchema` fail loud. `CANDI_NO_TUI=1` prints `page=<1-based>`, still saves sidecar. `candi_tui::run(doc, filename, initial) -> Result<ViewState, RunError>`; `TerminalGuard` unchanged. 10 CLI integration tests.
- **Slice 01/09 candi-tui merged to `dev`** (PR #14, https://github.com/mehboobulqadri/candi/pull/14): slice feat `238d410`, TerminalGuard `8ae4cc4`, deny Zlib `6f3c8c8`, merge `62739c9`, mergedAt 2026-08-20T19:32:23Z. ratatui 0.30.2, crossterm 0.29.0; §6 keys; `TestBackend`; Spike 2 frankl SKIPPED honest; `TerminalGuard` Drop RAII; `TERM=dumb` before raw mode; `candi-theme` dep dropped from tui. Independent reviewer APPROVE after REQUEST CHANGES (raw-mode). cargo-deny fail was foldhash Zlib — allowed in `deny.toml`.
- **Slices 01/01–01/08 on `dev`** (PRs #4–#13). **Cursor harness parity** (sync.sh, orchestration.mdc, remove.sh).

## In progress

Nothing active.

## Next

1. **Slice 01/11 — v01-release** (`docs/implementation/01-v0.1/slices/11-v01-release/README.md`) — next **engineering** slice only; **do not start** until delegated.
2. **01/11 stop-gates (user authorization required):** dogfood gate (daily use evidence before tag); tag `v0.1`; `dev` → `main` phase merge. Mia must not proceed without explicit user OK.
3. **01/11 engineering scope:** full benchmark re-run (both backends, `reader_peak` gate); REQUIREMENTS.md acceptance checklist; Windows CI job; version 0.1.0; independent worktree review of release diff; update `progress.md` before merge.
4. **Standing constraint:** slice workflow — one commit → 4-stage drill → independent reviewer in separate worktree → update `progress.md` before merge → merge to `dev` (01/11 merges to `main`).
5. **Optional (not next-required):** PR #1 still OPEN — user did not authorize merge.

## Open questions

- **Unsupported PDF-feature row** in hardening table — deferred in 01/04 (factory gating only).
- **MuPDF store cap / PDFium-for-full-doc-search** — deferred (not blocking 01/11).

## Nits carried forward

| Nit | Action |
|---|---|
| **`unix_days_to_ymd` (01/07)** | Hand-rolled calendar conversion untested — add unit tests or use a tested crate when touching dates. |
| **Windows atomic test (01/07)** | Vacuous pass — strengthen or document platform limit. |
| **`sync_dir` after rename (01/07)** | `save` succeeds but returns `Err` if dir fsync fails post-rename — document or tighten error semantics. |
| **`PDFIUM_OPS` mutex vs `thread_safe`** | Document rationale or prove redundant. |
| **Duplicate zero-page message strings** | Consolidate when touching backends next. |
| **`ViewState::scroll_down`** | Use `saturating_add` (01/06 nit). |
| **`ViewState::max_scroll` semantics** | Document inclusive max in API/docs (01/06 nit). |
| **`ViewState` scroll-reset tests** | Explicit first/last page coverage (01/06 nit). |
| **`progress.md` test count** | May still say 8 CLI tests — actual count is 10; update only at 01/11 merge close-out. |

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **CLI exit codes** | `Args::try_parse()` failures (usage, unknown flags) exit **1** — same as runtime errors; distinct stderr text per kind. |
| **Headless CLI tests** | `CANDI_NO_TUI=1` skips TUI, prints `page=<1-based>`, still saves sidecar — use in CI integration tests. |
| **Sidecar semantics** | Missing ≠ corrupt — `Load::Missing` vs `Load::Corrupt`; only schema > 1 is hard error. PDF path never modified. |
| **TUI terminal RAII** | Always restore terminal via `TerminalGuard` Drop after `enable_raw_mode`; set `TERM=dumb` before raw mode in tests. |
| **SearchSession wrap** | When `start_page > 0` and wrap enabled, scan must stop at `start_page` — never rescan the start page (01/08 bug class). |
| **RSS budget** | Gate **`reader_peak`** only; no v0.1 store purge. |
| **Fontconfig / native sysdeps** | MuPDF on Linux needs `libfontconfig1-dev` + `pkg-config` (PR #6). |
| **Path-dep feature unification** | CI pdfium-only jobs still compile MuPDF — path deps unify features. |
| **Prove guards with fixtures** | Unproven guards triggered REQUEST CHANGES on 01/02 and 01/04. |
| **Bench corpus** | Real books in gitignored `bench/corpus-local.toml` — never commit. |
| **cargo-deny licenses** | foldhash/hashbrown pulls Zlib — allow in `deny.toml` or deny fails (01/09). |
| **01/11 dogfood** | v0.1 tag blocked until daily-use evidence — cannot be forced. |
