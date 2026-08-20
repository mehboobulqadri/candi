# Phase 01 — v0.1 (Core + TUI)

## Goal

The minimum viable reader (workflow Phase 2): `candi book.pdf` opens, displays
text, scrolls, paginates, searches with next/prev, and remembers reading position
(project.md 4.1). Entry: phase 00 exit.

## What gets built

- **candi-pdf** — trait + error model, MuPDF and PDFium engines (feature-gated,
  runtime-switchable), parity suite, benchmarks.
- **candi-core** — navigation model, reading-position sidecar, lazy search
  abstraction.
- **candi-tui** — keyboard-first terminal UI (ratatui/crossterm), SSH-safe.
- **candi-cli** — `candi book.pdf [--backend mupdf|pdfium]` with explicit error UX.
- **Spike 2 closure** — text-first rendering validated on a single-column novel;
  multi-column/footnote limitations documented (in slice 09).

## Exit criteria

- v0.1 acceptance criteria (REQUIREMENTS.md §v0.1) all pass.
- Benchmarks on both backends meet the architecture.md §9 budget: startup-to-first
  page < 300 ms, open ≤ 150 ms, page-text < 20 ms/page mean, first search result
  < 300 ms, next/prev < 50 ms, peak RSS < 200 MB.
- Windows build verified (CI job or local) at the tag.
- **Dogfood gate** — used daily by at least one person *before* tagging
  (workflow Phase 2 decision gate).

## Slice index

| # | Slice | Status |
|---|---|---|
| 01 | candi-pdf-trait | planned |
| 02 | mupdf-backend | planned |
| 03 | pdfium-backend | planned |
| 04 | backend-parity-hardening | planned |
| 05 | benchmarks-both-backends | planned |
| 06 | candi-core-navigation | planned |
| 07 | reading-position-sidecar | planned |
| 08 | search-abstraction | planned |
| 09 | candi-tui | planned |
| 10 | candi-cli | planned |
| 11 | v01-release | planned |

Each slice merges to `dev`; [progress.md](../progress.md) updates before every
merge. Slice READMEs live in `slices/<NN-name>/`.