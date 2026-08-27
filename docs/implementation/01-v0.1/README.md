# Phase 01 — v0.1 (Core + TUI + GUI text)

## Goal

The minimum viable reader (workflow Phase 2): open a PDF, display text, scroll,
paginate, search with next/prev, and remember reading position (project.md 4.1).
Entry: phase 00 exit.

GUI text display was pulled into v0.1 (compressed scope); pixel PDF page rendering
(images, diagrams) remains a later phase.

## What gets built

- **candi-pdf** — trait + error model, MuPDF and PDFium engines (feature-gated,
  runtime-switchable), parity suite, benchmarks.
- **candi-core** — navigation model, reading-position sidecar, lazy search
  abstraction.
- **candi-tui** — keyboard-first terminal UI (ratatui/crossterm), SSH-safe.
- **candi-gui** — `candi` binary: egui text reader with file dialog.
- **candi-cli** — shared lib: open document, sidecar resume/save (not a user binary).
- **Spike 2 closure** — text-first rendering validated on a single-column novel;
  multi-column/footnote limitations documented (in slice 09).

## Exit criteria

- v0.1 acceptance criteria (REQUIREMENTS.md §v0.1) all pass.
- Benchmarks on both backends meet the architecture.md §Cross-cutting
  performance budget.
- **Dogfood gate** — used daily by at least one person *before* tagging
  (workflow Phase 2 decision gate).

## Slice index

| # | Slice | Status |
|---|---|---|
| 01 | candi-pdf-trait | merged |
| 02 | mupdf-backend | merged |
| 03 | pdfium-backend | merged |
| 04 | backend-parity-hardening | merged |
| 05 | benchmarks-both-backends | merged |
| 06 | candi-core-navigation | merged |
| 07 | reading-position-sidecar | merged |
| 08 | search-abstraction | merged |
| 09 | candi-tui | merged |
| 10 | candi-cli | merged |
| 11 | v01-release | merged |

Post-01/11 on `dev`: **tui-readability** (PR #17), **gui-text-linux** (PR #18).

Slice READMEs live in `slices/<NN-name>/`.
