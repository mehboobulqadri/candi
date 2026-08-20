# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal document reader (TUI + GUI text) on a shared Rust core; v0.1 on `dev`
with dual PDF backends (MuPDF default, PDFium permissive).

## Current focus

**Reader shipped on `dev`** (PR #18, HEAD `54d9f26`). Binaries: `candi-tui`, `candi`
(GUI). Next: user stop-gates — dogfood, tag `v0.1`, `dev`→`main`.

## Active constraints

- **Stop-gates:** dogfood, tag, `main` merge — explicit user authorization required.
- **Platforms:** Linux dogfood; Windows CI deferred (Linux-only matrix).
- **Entry points:** `candi-tui book.pdf`; `candi` / `candi book.pdf [--backend …]`.
- **`candi-cli`:** internal lib only; `CANDI_NO_GUI=1` is test/CI hook.
- **RSS budget:** gate **`reader_peak`** only.

## Stack

Rust 1.97.1, MuPDF 0.8.0, PDFium `pdfium-render 0.8.37`, ratatui 0.30.2, egui 0.30,
GitHub Actions + cargo-deny.
