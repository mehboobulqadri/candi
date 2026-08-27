# Candi — a minimal, fast, cross-platform document reader

A document reader with a TUI and a GUI.

> One reader. One core. Two interfaces.

## Status

v0.1 is feature-complete on `slice/02-01-gui-reader` pending tag: terminal UI
(`candi-tui`) and graphical UI (`candi`) with real rendered PDF pages. Dual PDF backends
(MuPDF default, PDFium permissive build). Linux dogfooding; not yet tagged or on `main`.

License: AGPL-3.0, with a permissive-build escape hatch (pdfium-only feature build).
[Architecture](docs/architecture.md) covers design decisions;
[progress](docs/implementation/progress.md) tracks where we are (release gates —
dogfood, tag, `main` — need explicit user authorization).

## What you get

- **`candi`** — GUI (egui). No args opens a file dialog; `candi book.pdf` opens that file:
  - Rendered PDF pages with continuous scroll, single-page, dual-page spreads, fit-page /
    fit-width flows, anchor-preserving zoom and pinch.
  - Search across the document with in-page highlights; table-of-contents sidebar;
    bookmarks with rename.
  - Appearance panel: 5 built-in YAML themes, an in-app theme editor, accent retint, and
    custom themes from `~/.config/candi/themes/*.yaml`.
  - Editable keybindings (`~/.config/candi/keybinds.json`, seeded with the defaults),
    recent-files list, per-PDF resume via a sidecar next to each book, drag-and-drop open.
  - Optional `--backend mupdf|pdfium` (default `mupdf`).
- **`candi-tui book.pdf`** — keyboard-first terminal reader (ligature-normalized text,
  centered column, mouse wheel and j/k page scroll) — works over SSH.

Shared open/resume/sidecar logic lives in the `candi-cli` library crate, wired by both
frontends.

## Building from source (Arch-based)

```bash
sudo pacman -S --needed rust clang fontconfig pkg-config
cargo build --release
```

`clang` is needed once, to compile `mupdf-sys`'s bundled copy of MuPDF's C sources
(statically linked — no system `mupdf` package is used); `fontconfig` and
`pkg-config` satisfy its Linux link requirements.

This produces two binaries:

| Binary | Run |
|---|---|
| `target/release/candi` | the GUI |
| `target/release/candi-tui` | the terminal UI |

No prebuilt PDFium download is needed for a source build (MuPDF is the default backend);
the optional permissive PDFium build is described in
[docs/architecture.md](docs/architecture.md). Desktop entries can point at either binary —
set `StartupWMClass=candi` so the window groups correctly.

Or install straight from a clone:

```bash
cargo install --path crates/candi-gui   # binary: candi
cargo install --path crates/candi-tui   # binary: candi-tui
```

## Roadmap

Versioned releases; feature work merges to `dev`, `dev` merges to `main` for major
releases. Full rules and details: [docs/roadmap.md](docs/roadmap.md).

- **v0.1** — shipped base + UI fixes: TUI reader, GUI reader, dual backends, themes,
  bookmarks, custom themes & keybinds.
- **v0.2** — EPUB & other formats, password-locked PDFs, optimizations/cleanup.
- **v0.3** — multi-OS (Windows, other Linux distros), deployments.
- **v0.4** — GitHub documentation site + cleaning.
- **v0.5** — AUR/Debian packaging + CI.
- **v0.6** — Android (beta), Android UI optimization, CI.

## Project documents

- [Roadmap & versioning rules](docs/roadmap.md)
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
