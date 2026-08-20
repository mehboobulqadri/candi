# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal, fast, cross-platform document reader (TUI + Slint GUI) on a shared native Rust core; v0.1 goal is excellent PDF reading via dual backends (MuPDF default, PDFium permissive).

## Current focus

**Phase 01 code complete on `dev`** (01/11 merged, PR #16, merge `638f8bc`). **Next: user-authorized release gates** — dogfood evidence, tag `v0.1`, `dev`→`main`. Not more engineering slices.

## Active constraints

- **Stop-gates:** dogfood, tag, `main` merge — explicit user authorization required; Mia must not proceed alone.
- Slice workflow complete for Phase 01; future work follows same drill if new slices appear.
- **CI:** rust-checks matrix ubuntu + windows × default + pdfium-only; MuPDF builds on Windows; Linux-only bench/fontconfig.
- **CLI:** `candi FILE [--backend mupdf|pdfium]`; `CANDI_NO_TUI=1` for headless CI.
- **RSS budget:** gate **`reader_peak`** only; no v0.1 store purge.
- PDFium: `chromium/7543` via `pdfium-render 0.8.37`; MuPDF TLS `fz_context`.

## Stack

Rust 1.97.1 (pinned), workspace crates, MuPDF 0.8.0, PDFium via `pdfium-render 0.8.37`, ratatui 0.30.2, GitHub Actions + cargo-deny.
