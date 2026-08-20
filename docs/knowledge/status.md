# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal, fast, cross-platform document reader (TUI + Slint GUI) on a shared native Rust core; v0.1 goal is excellent PDF reading via dual backends (MuPDF default, PDFium permissive).

## Current focus

**Slice 01/11 — v01-release** next (`docs/implementation/01-v0.1/slices/11-v01-release/README.md`). **01/10 candi-cli merged** (PR #15, merge `9b86661`). Phase 01 **one slice left** — 01/11 engineering only; tag/dogfood/`dev`→`main` need user.

## Active constraints

- Slice workflow: one commit on `slice/NN-name` off `dev`; 4-stage drill + independent worktree review; update `progress.md` before merge.
- **01/11 stop-gates:** dogfood evidence, tag `v0.1`, `dev`→`main` — explicit user authorization required.
- Mia approves worker permissions (network/sandbox/git_write/slice-merge); no user stall unless destructive git or 01/11 stop-gates.
- **CLI:** `candi FILE [--backend mupdf|pdfium]`; parse failures exit 1; `CANDI_NO_TUI=1` for headless CI; sidecar resume via `ViewState::goto_page`.
- **TUI:** ratatui 0.30.2, crossterm 0.29.0; `TerminalGuard` RAII; `candi_tui::run(doc, filename, initial)`.
- **RSS budget:** gate **`reader_peak`** only; no v0.1 store purge.
- Sidecar: `{pdf}.candi.toml`, schema v1; missing ≠ corrupt; atomic temp+rename; PDF never written.
- PDFium: `Arc<Pdfium>`, `pdfium-render 0.8.37` with `sync` + `thread_safe`, libpdfium `chromium/7543`.
- MuPDF: TLS `fz_context`; CI fontconfig dev packages; path-dep feature unification still builds both backends on pdfium matrix jobs.
- cargo-deny: allow Zlib for foldhash/hashbrown transitive (01/09).

## Stack

Rust 1.97.1 (pinned), workspace crates (`candi-core`, `candi-pdf`, `candi-tui`, `candi-cli`, …), MuPDF 0.8.0, PDFium via `pdfium-render 0.8.37`, ratatui 0.30.2, GitHub Actions + cargo-deny.
