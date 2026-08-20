# Candi — a minimal, fast, cross-platform document reader

A document reader with a TUI and a GUI.

> One reader. One core. Two interfaces.

## Status

v0.1 text reader ships on `dev`: terminal UI (`candi-tui`) and graphical UI (`candi`).
Dual PDF backends (MuPDF default, PDFium permissive build). Linux dogfood; not tagged,
not on `main`, no packaging yet.

- Spike 1 (PDF backend) resolved: MuPDF + PDFium ship in v1 as a dual, runtime-switchable
  backend behind one trait; Poppler rejected. License: AGPL-3.0, with a permissive-build
  escape hatch (pdfium-only feature build).
- [Architecture](docs/architecture.md) guides the workspace layout and design decisions.
- [Implementation](docs/implementation/README.md) — phase/slice plan;
  [progress](docs/implementation/progress.md) tracks where we are (Phase 01 merged on
  `dev`; release gates — dogfood, tag, `main` — need explicit user authorization).

## Quick start

Install (from a clone):

```bash
cargo install --path crates/candi-gui   # binary: candi
cargo install --path crates/candi-tui   # binary: candi-tui
```

Run:

- **`candi`** — GUI (egui). No args opens a file dialog; `candi book.pdf` opens that
  file. Scrollable extracted text, page navigation, search, reading-position sidecar.
  Optional `--backend mupdf|pdfium` (default `mupdf`).
- **`candi-tui book.pdf`** — keyboard-first terminal reader (ligature-normalized text,
  centered column, mouse wheel and j/k page scroll).

There is no user-facing CLI reader binary. Shared open/resume/sidecar logic lives in the
`candi-cli` library crate, wired by both frontends.

## Project documents

- [Project](docs/project.md)
- [Requirements](docs/REQUIREMENTS.md)
- [Spikes and options](docs/spikes.md)
- [Workflow](docs/workflow.md)
- [Architecture](docs/architecture.md)
- [Implementation](docs/implementation/README.md) — phase/slice build plan and working method

Public documentation sources live under `docs/`. The site is published with
GitHub Pages — see [docs/publishing.md](docs/publishing.md) for how.

## Maintainer

Mehboob ul Qadri — [mehboobulqadri@gmail.com](mailto:mehboobulqadri@gmail.com)
