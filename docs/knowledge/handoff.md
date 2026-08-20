# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Slice 01/03 pdfium-backend merged to `dev`** (PR #7, https://github.com/mehboobulqadri/candi/pull/7): feat `b3d1326`, merge `b9902d0`. `PdfiumBackend`, `Arc<Pdfium>` factory, `FPDF_DOCUMENT` drop, `FPDF_ERR_*` mapping, `u16::try_from` guards, permissive build (`--no-default-features --features pdfium-backend`). CI: libpdfium fetch/cache + sha256 pin (bblanchon `chromium/7543`, `pdfium_7543`); `PDFIUM_LIB` on test jobs. PR CI run `32383125922`: all 7 jobs success. Post-merge CI run `32384136200` on `b9902d0`: all 7 jobs success.
- **Independent reviewer APPROVE** (nits only) — worktree `/tmp/candi-slice-01-03-pdfium-backend-review`.
- **Prior slices still on `dev`:** 01/02 mupdf-backend (PR #5), CI fontconfig hotfix (PR #6, merge `98a4744`, post-merge CI `32379667373` green).
- **Cursor harness parity** (sync.sh, orchestration.mdc, remove.sh).

## In progress

**Slice 01/04 — backend-parity-hardening** starting (`docs/implementation/01-v0.1/slices/04-backend-parity-hardening/README.md`). Builder sets `progress.md` row for 01/03 merged + starts 01/04 — do not edit `progress.md` in this close-out.

## Next

1. Complete slice **01/04** (backend-parity): formalize cross-backend fixture expectations; address reviewer nits carried from 01/03 (see below).
2. **Slice cadence 04 → 11** sequentially — user override this session: complete remaining Phase 01 slices in order; do not skip ahead.
3. Full slice workflow per slice: one independent commit → in-session 4-stage drill → independent reviewer in separate worktree → update `docs/implementation/progress.md` before merge → merge to `dev` (Mia may approve slice PR merge to `dev` under Phase-01 cadence; do not stall for user).
4. **Standing constraint:** Mia approves worker permission requests (`full_network`, sandbox `all`, `git_write`, slice merge to `dev`) — do not bounce to user unless destructive git (force-push, hard reset) or 01/11 tag/dogfood/main-merge.
5. **01/11 dogfood / tag / `main` merge** — needs explicit user authorization; not auto-next.
6. **Optional (not next-required):** PR #1 still OPEN — user did not authorize merge.

## Open questions

- None blocking 01/04.

## Nits carried into 01/04 (from 01/03 reviewer)

| Nit | Action |
|---|---|
| **Catalog sniff is fixture-specific** | `is_zero_page_catalog` bridges PDFium `FORMAT` vs MuPDF `page_count == 0` on `zero-pages.pdf` — harden for general PDFs or document explicitly in parity slice. |
| **`PDFIUM_OPS` mutex vs `thread_safe`** | `thread_safe` alone did not stop parallel-test SIGABRT; global mutex added — document rationale or prove redundant. |
| **No libpdfium on bench until needed** | YAGNI: do not add libpdfium install to `bench` job until slice 01/05 needs it. |

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **Fontconfig / native sysdeps** | MuPDF on Linux pulls `yeslogic-fontconfig-sys`. CI needs `libfontconfig1-dev` + `pkg-config` (PR #6). See skill `ci-reliability` §5. |
| **Path-dep feature unification** | CI matrix “pdfium-only” jobs still compile MuPDF because workspace path deps unify features across crates. Do not assume `--no-default-features --features pdfium-backend` isolates MuPDF on CI until matrix/workspace layout changes. |
| **TLS vs Arc** | MuPDF uses TLS `fz_context` (crate reality); PDFium uses `Arc<Pdfium>` per architecture.md — keep docs/code honest. |
| **Prove guards with fixtures** | Unproven guards triggered round-1 REQUEST CHANGES on 01/02; 01/03 zero-page parity needed catalog sniff — prove behavior, do not assert. |
| **libpdfium pin** | `pdfium-render 0.8.37` `pdfium_latest` = `pdfium_7543` (bblanchon `chromium/7543`, sha256 `2383a414…`). Slice README marketing version `153.0.8009.0` is NOT the ABI pin — use chromium/7543. |
| **pdfium-render `sync` feature** | Required for `Arc<Pdfium>` and `Document` Send+Sync — enable in `Cargo.toml` with `pdfium_latest` + `thread_safe`. |
| **Post-merge CI** | Run `32384136200` on merge `b9902d0`: all 7 jobs success (verified). |
