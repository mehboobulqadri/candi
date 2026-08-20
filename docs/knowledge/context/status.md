# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal, fast, cross-platform document reader (TUI + Slint GUI) on a shared native Rust core; v0.1 goal is excellent PDF reading via dual backends (MuPDF default, PDFium permissive).

## Current focus

**Slice 01/06 — candi-core-navigation** next (`docs/implementation/01-v0.1/slices/06-candi-core-navigation/README.md`). **01/05 benchmarks-both-backends merged** (PR #10, merge `fc5a2af`). Phase 01 **not complete** — slices 06–11 remain. Optional: PDF-reader RSS research (not a slice).

## Active constraints

- Slice workflow: one commit on `slice/NN-name` off `dev`; 4-stage drill + independent worktree review; update `progress.md` before merge.
- User override: complete remaining Phase 01 slices 06–11 sequentially; 01/11 tag/dogfood/main-merge needs user.
- Mia approves worker permissions (network/sandbox/git_write/slice-merge-to-dev); no user stall unless destructive git or 01/11 main-merge.
- **RSS budget:** gate **`reader_peak`** (open + first page + search/nav window); **`full_pass_peak`** (full page sweep) printed not gated — MuPDF ~295 MB = `FZ_STORE_DEFAULT` in TLS `fz_context`.
- Bench: `crates/candi-pdf/benches/bench.rs` + `bench/run.sh`; CI `--fixtures-only`, `BENCH_CHECK_BUDGET=0`; corpus gitignored (`bench/corpus-local.toml`).
- PDFium: `Arc<Pdfium>`, `pdfium-render 0.8.37` with `sync` + `thread_safe`, libpdfium `chromium/7543` via `PDFIUM_LIB`.
- MuPDF: TLS `fz_context` (honest); CI fontconfig dev packages (PR #6); path-dep feature unification still builds both backends on pdfium matrix jobs.

## Stack

Rust 1.97.1 (pinned), workspace crates (`candi-pdf`, …), MuPDF 0.8.0 (`mupdf-backend`), PDFium via `pdfium-render 0.8.37` (`pdfium-backend`), GitHub Actions + cargo-deny.
