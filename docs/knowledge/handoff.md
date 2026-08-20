# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Slice 01/06 candi-core-navigation merged to `dev`** (PR #11, https://github.com/mehboobulqadri/candi/pull/11): slice feat `8b955fa`, progress `e44a58d`, merge `994a154`. `ViewState` in `candi-core`: 0-based page; clamp never `Result`/panic on realistic inputs; `max_scroll` caller-supplied (no TUI types); page change resets scroll to 0; empty doc stays page 0; `Copy`. 15 tests. CI run `32401474027` on slice HEAD: all jobs success. Independent reviewer APPROVE. Remote slice branch deleted; worktree `/tmp/candi-slice-01-06` removed.
- **Slice 01/05 benchmarks-both-backends merged to `dev`** (PR #10, https://github.com/mehboobulqadri/candi/pull/10): slice HEAD `883a98c`, gate-fix `1fb4242`, merge `fc5a2af`. Production harness: `crates/candi-pdf/benches/bench.rs` + `bench/run.sh`; both backends; corpus via gitignored `bench/corpus-local.toml` (never commit real books). CI `--fixtures-only`, `BENCH_CHECK_BUDGET=0`; `nav_ms` = adjacent `page_text` proxy until 01/08. CI run `32393653573` on `883a98c`: all 7 jobs success. Independent reviewer APPROVE at `883a98c`. Remote slice branch deleted; worktree `/tmp/candi-slice-01-05` removed.
- **RSS methodology (01/05):** **`reader_peak`** (open + first page + search/nav page window) is the **v0.1 200 MB gate** — MuPDF silberschatz ~43–44 MB. **`full_pass_peak`** after full page sweep is **printed, not gated** — MuPDF silberschatz ~295 MB (= `FZ_STORE_DEFAULT` 256 MiB decode store in TLS `fz_context`; dropping `Page` does not empty the store). Do **not** claim lazy loading “fixed 294→43”; that was a measurement-window artifact.
- **PDF-reader RSS research complete** (no code change). Real readers use **two layers**: (1) app keep-N page window (Zathura 16, pdf.js 10 / 2×visible+1, Sioyek 5, Chromium Unload non-visible, Sumatra 128 bitmaps); (2) engine decode store (MuPDF `FZ_STORE_DEFAULT` 256 MiB LRU, survives `fz_drop_page`). Nobody serious calls `fz_empty_store` per page (Candi probe ~940 MB worse). **Recommendation:** **no v0.1 RSS implementation** — keep lazy `page_text`; gate **`reader_peak`** only (~44 MB already “good reader”); v0.3 copy keep-N window; optional later `mupdf-sys` store cap or PDFium for full-doc search. `mupdf-rs` 0.8.0 does not expose `max_store` (C API does; llpp exposes `mupdf-store-size`).
- **Slice 01/04 backend-parity-hardening merged** (PR #8, slice `153cafc`, merge `3d519af`). Shared parity suite, hardening matrix, first-page text sampling.
- **Prior slices on `dev`:** 01/01–01/03 (PRs #4–#7); PR #9 identity merged `b3abfe5`.
- **Cursor harness parity** (sync.sh, orchestration.mdc, remove.sh).

## In progress

Nothing active.

## Next

1. **Slice 01/07 — reading-position-sidecar** (`docs/implementation/01-v0.1/slices/07-reading-position-sidecar/README.md`) — next **code** slice; **do not start** until delegated.
2. **Slice cadence 07 → 11** sequentially — user override: complete remaining Phase 01 slices in order; do not skip ahead.
3. Full slice workflow per slice: one independent commit → in-session 4-stage drill → independent reviewer in separate worktree → update `docs/implementation/progress.md` before merge → merge to `dev` (Mia may approve slice PR merge to `dev` under Phase-01 cadence; do not stall for user).
4. **Standing constraint:** Mia approves worker permission requests (`full_network`, sandbox `all`, `git_write`, slice merge to `dev`) — do not bounce to user unless destructive git (force-push, hard reset) or 01/11 tag/dogfood/main-merge.
5. **01/11 dogfood / tag / `main` merge** — needs explicit user authorization; not auto-next.
6. **Optional (not next-required):** PR #1 still OPEN — user did not authorize merge.

## Open questions

- **Unsupported PDF-feature row** in hardening table — deferred in 01/04 (factory gating only; no open-time Unsupported on PDFium). Decide fixture/policy in a later slice or accept factory-only gate.
- **MuPDF store cap / PDFium-for-full-doc-search** — deferred (not blocking 01/07): `mupdf-rs` 0.8.0 lacks `max_store`; would need `mupdf-sys` fork or alternate backend for search-heavy full-doc paths.

## Nits carried forward

| Nit | Action |
|---|---|
| **`PDFIUM_OPS` mutex vs `thread_safe`** | `thread_safe` alone did not stop parallel-test SIGABRT; global mutex added — document rationale or prove redundant. |
| **Duplicate zero-page message strings** | Same literal in `mupdf.rs`, `pdfium.rs`, and test files — consolidate when touching backends next. |
| **Slice README wording** | 01/05 README “All budget targets met” could imply `full_pass_peak` is gated — wording nit only; gate is `reader_peak`. |
| **`ViewState::scroll_down`** | Use `saturating_add` instead of plain add (01/06 reviewer nit, non-blocking). |
| **`ViewState::max_scroll` semantics** | Document inclusive max in API/docs (01/06 reviewer nit). |
| **`ViewState` scroll-reset tests** | Add explicit first/last page scroll-reset coverage (01/06 reviewer nit). |

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **Fontconfig / native sysdeps** | MuPDF on Linux pulls `yeslogic-fontconfig-sys`. CI needs `libfontconfig1-dev` + `pkg-config` (PR #6). See skill `ci-reliability` §5. |
| **Path-dep feature unification** | CI matrix “pdfium-only” jobs still compile MuPDF because workspace path deps unify features across crates. Do not assume `--no-default-features --features pdfium-backend` isolates MuPDF on CI until matrix/workspace layout changes. |
| **TLS vs Arc** | MuPDF uses TLS `fz_context` (crate reality); PDFium uses `Arc<Pdfium>` per architecture.md — keep docs/code honest. |
| **Prove guards with fixtures** | Unproven guards triggered round-1 REQUEST CHANGES on 01/02 and 01/04 — prove behavior, do not assert. |
| **libpdfium pin** | `pdfium-render 0.8.37` `pdfium_latest` = `pdfium_7543` (bblanchon `chromium/7543`, sha256 `2383a414…`). Slice README marketing version `153.0.8009.0` is NOT the ABI pin — use chromium/7543. |
| **pdfium-render `sync` feature** | Required for `Arc<Pdfium>` and `Document` Send+Sync — enable in `Cargo.toml` with `pdfium_latest` + `thread_safe`. |
| **progress.md SHA lag** | Recurring — write progress row only after slice commit is final AND update at merge close-out; 01/05 added docs commit `883a98c` after gate fix `1fb4242` (correct pattern vs amending). |
| **RSS budget semantics** | Gate **`reader_peak`** only; **`full_pass_peak`** recorded not gated. Do not market 294→43 as optimization. Do not `fz_empty_store` per page in production. |
| **Bench corpus** | Real books live in gitignored `bench/corpus-local.toml` — never commit. |
| **`ViewState` navigation semantics** | 0-based page; clamp invariants never panic/`Result` on realistic inputs; `max_scroll` caller-supplied; page change resets scroll to 0; empty doc stays page 0. |
