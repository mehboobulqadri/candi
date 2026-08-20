# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Slice 01/04 backend-parity-hardening merged to `dev`** (PR #8, https://github.com/mehboobulqadri/candi/pull/8): slice HEAD `153cafc`, merge `3d519af`. Shared parity suite (`tests/parity/`), hardening matrix, first-page text sampling (`textlayer.rs`), fixture helpers. CI run `32387915511` on `153cafc`: all 7 jobs success.
- **Independent reviewer APPROVE** (round 2b). Round 1 REQUEST CHANGES: `_open_fn` typo; dishonest Unsupported self-match. Fixes: Option B Unsupported (factory gating only — PDFium never maps open to Unsupported); textlayer zero-page → `Malformed`; compile skip `let _ = open_fn`.
- **Prior slices on `dev`:** 01/01–01/03 (PRs #4–#7); PR #9 identity merged `b3abfe5`.
- **Cursor harness parity** (sync.sh, orchestration.mdc, remove.sh).

## In progress

**Slice 01/05 — benchmarks-both-backends** next (`docs/implementation/01-v0.1/slices/05-benchmarks-both-backends/README.md`). Builder sets `progress.md` row for 01/04 merged + merge SHA — do not edit `progress.md` in this close-out (`progress.md` still cites stale SHA `1b4b613`).

## Next

1. Complete slice **01/05** (benchmarks): wire both backends into production harness; full corpus; compare against architecture.md §Cross-cutting budget; update spike doc.
2. **Slice cadence 05 → 11** sequentially — user override this session: complete remaining Phase 01 slices in order; do not skip ahead.
3. Full slice workflow per slice: one independent commit → in-session 4-stage drill → independent reviewer in separate worktree → update `docs/implementation/progress.md` before merge → merge to `dev` (Mia may approve slice PR merge to `dev` under Phase-01 cadence; do not stall for user).
4. **Standing constraint:** Mia approves worker permission requests (`full_network`, sandbox `all`, `git_write`, slice merge to `dev`) — do not bounce to user unless destructive git (force-push, hard reset) or 01/11 tag/dogfood/main-merge.
5. **01/11 dogfood / tag / `main` merge** — needs explicit user authorization; not auto-next.
6. **Optional (not next-required):** PR #1 still OPEN — user did not authorize merge.

## Open questions

- **Unsupported PDF-feature row** in hardening table — deferred in 01/04 (factory gating only; no open-time Unsupported on PDFium). Decide fixture/policy in a later slice or accept factory-only gate.
- None blocking 01/05.

## Nits carried forward

| Nit | Action |
|---|---|
| **`PDFIUM_OPS` mutex vs `thread_safe`** | `thread_safe` alone did not stop parallel-test SIGABRT; global mutex added — document rationale or prove redundant. |
| **Duplicate zero-page message strings** | Same literal in `mupdf.rs`, `pdfium.rs`, and test files — consolidate when touching backends next. |
| **libpdfium on bench job** | 01/05 needs both backends — add libpdfium install to `bench` job if not already present (01/03 YAGNI lifted). |

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **Fontconfig / native sysdeps** | MuPDF on Linux pulls `yeslogic-fontconfig-sys`. CI needs `libfontconfig1-dev` + `pkg-config` (PR #6). See skill `ci-reliability` §5. |
| **Path-dep feature unification** | CI matrix “pdfium-only” jobs still compile MuPDF because workspace path deps unify features across crates. Do not assume `--no-default-features --features pdfium-backend` isolates MuPDF on CI until matrix/workspace layout changes. |
| **TLS vs Arc** | MuPDF uses TLS `fz_context` (crate reality); PDFium uses `Arc<Pdfium>` per architecture.md — keep docs/code honest. |
| **Prove guards with fixtures** | Unproven guards triggered round-1 REQUEST CHANGES on 01/02 and 01/04 — prove behavior, do not assert. |
| **libpdfium pin** | `pdfium-render 0.8.37` `pdfium_latest` = `pdfium_7543` (bblanchon `chromium/7543`, sha256 `2383a414…`). Slice README marketing version `153.0.8009.0` is NOT the ABI pin — use chromium/7543. |
| **pdfium-render `sync` feature** | Required for `Arc<Pdfium>` and `Document` Send+Sync — enable in `Cargo.toml` with `pdfium_latest` + `thread_safe`. |
| **progress.md SHA lag** | Recurring — write progress row only after slice commit is final; 01/04 builder on 01/05 must fix stale `1b4b613`. |
