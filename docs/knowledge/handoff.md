# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Slice 01/08 search-abstraction merged to `dev`** (PR #13, https://github.com/mehboobulqadri/candi/pull/13): slice feat `9c7e0f1`, wrap fix `a77a874`, progress `7f6655a`, merge `4af89e8`. `SearchSession`: lazy page-at-a-time, case-insensitive `to_lowercase`, results `(page, offset)` only; `start_page` + wrap; empty query → 0 `page_text` calls; errors propagate. Wrap bug (start_page>0 rescanned start page) fixed in `a77a874` — test asserts `call_count == page_count`. CI run `32404983245`. Independent reviewer APPROVE after REQUEST CHANGES (wrap rescan).
- **Slice 01/07 reading-position-sidecar merged to `dev`** (PR #12, merge `decce47`). Sidecar `{pdf}.candi.toml`, schema v1; `Load::{Missing,Loaded,Corrupt}`; atomic temp+rename; PDF never written.
- **Slice 01/06 candi-core-navigation merged to `dev`** (PR #11, merge `994a154`). `ViewState` in `candi-core`.
- **Slice 01/05 benchmarks-both-backends merged to `dev`** (PR #10, merge `fc5a2af`). Gate **`reader_peak`** only.
- **Prior slices on `dev`:** 01/01–01/04 (PRs #4–#8); PR #9 identity merged `b3abfe5`.
- **Cursor harness parity** (sync.sh, orchestration.mdc, remove.sh).

## In progress

Nothing active.

## Next

1. **Slice 01/09 — candi-tui** (`docs/implementation/01-v0.1/slices/09-candi-tui/README.md`) — next **code** slice; **do not start** until delegated.
2. **Slice cadence 09 → 11** sequentially — user override: complete remaining Phase 01 slices in order.
3. Full slice workflow per slice: one independent commit → 4-stage drill → independent reviewer in separate worktree → update `docs/implementation/progress.md` before merge → merge to `dev`.
4. **Standing constraint:** Mia approves worker permissions (`full_network`, sandbox `all`, `git_write`, slice merge to `dev`); do not stall for user unless destructive git or 01/11 tag/dogfood/main-merge.
5. **01/11 dogfood / tag / `main` merge** — needs explicit user authorization.
6. **Optional (not next-required):** PR #1 still OPEN — user did not authorize merge.

## Open questions

- **Unsupported PDF-feature row** in hardening table — deferred in 01/04 (factory gating only).
- **MuPDF store cap / PDFium-for-full-doc-search** — deferred (not blocking 01/09).

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

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **SearchSession wrap** | When `start_page > 0` and wrap enabled, scan must stop at `start_page` — never rescan the start page (01/08 bug class). |
| **Sidecar semantics** | Missing ≠ corrupt — `Load::Missing` vs `Load::Corrupt`; only schema > 1 is hard error. PDF path never modified. |
| **RSS budget** | Gate **`reader_peak`** only; no v0.1 store purge. |
| **Fontconfig / native sysdeps** | MuPDF on Linux needs `libfontconfig1-dev` + `pkg-config` (PR #6). |
| **Path-dep feature unification** | CI pdfium-only jobs still compile MuPDF — path deps unify features. |
| **Prove guards with fixtures** | Unproven guards triggered REQUEST CHANGES on 01/02 and 01/04. |
| **Bench corpus** | Real books in gitignored `bench/corpus-local.toml` — never commit. |
