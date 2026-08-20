# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal, fast, cross-platform document reader (TUI + Slint GUI) on a shared native Rust core; v0.1 goal is excellent PDF reading via dual backends (MuPDF default, PDFium permissive).

## Current focus

**Slice 01/09 — candi-tui** next (`docs/implementation/01-v0.1/slices/09-candi-tui/README.md`). **01/08 search-abstraction merged** (PR #13, merge `4af89e8`). Phase 01 **not complete** — slices 09–11 remain.

## Active constraints

- Slice workflow: one commit on `slice/NN-name` off `dev`; 4-stage drill + independent worktree review; update `progress.md` before merge.
- User override: complete remaining Phase 01 slices 09–11 sequentially; 01/11 tag/dogfood/main-merge needs user.
- Mia approves worker permissions (network/sandbox/git_write/slice-merge-to-dev); no user stall unless destructive git or 01/11 main-merge.
- **RSS budget:** gate **`reader_peak`** only; **`full_pass_peak`** printed not gated — no v0.1 store purge.
- **SearchSession:** lazy page-at-a-time; case-insensitive; `(page, offset)` hits; empty query skips `page_text`; wrap scan stops at `start_page` (no rescan).
- Sidecar: `{pdf}.candi.toml`, schema v1; missing ≠ corrupt; atomic temp+rename; PDF never written.
- PDFium: `Arc<Pdfium>`, `pdfium-render 0.8.37` with `sync` + `thread_safe`, libpdfium `chromium/7543`.
- MuPDF: TLS `fz_context`; CI fontconfig dev packages; path-dep feature unification still builds both backends on pdfium matrix jobs.

## Stack

Rust 1.97.1 (pinned), workspace crates (`candi-core`, `candi-pdf`, …), MuPDF 0.8.0, PDFium via `pdfium-render 0.8.37`, GitHub Actions + cargo-deny.
