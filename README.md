# Candi — a minimal, fast, cross-platform document reader

A clean, minimal, distraction-free PDF reader.

> One reader. One core. One interface.

## Status

v0.1 is feature-complete on `slice/02-01-gui-reader` pending tag: the graphical
reader (`candi`) with real rendered PDF pages. Dual PDF backends
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

Shared open/resume/sidecar logic lives in the `candi-cli` library crate, wired by the GUI
frontend.

## Installing

Three paths. All need the repo to be public — downloads, `cargo install --git`,
and clones 404 while it is private.

### Download (easiest)

Grab `candi-<ver>-x86_64-linux-gnu.tar.gz` and its `.sha256` sidecar from the
[Releases](https://github.com/mehboobulqadri/candi/releases) page:

```bash
sha256sum -c candi-<ver>-x86_64-linux-gnu.tar.gz.sha256
tar -xzf candi-<ver>-x86_64-linux-gnu.tar.gz
cd candi-<ver>-x86_64-linux-gnu
./install.sh   # installs binary, desktop entry, icons to ~/.local (override: PREFIX=/usr ./install.sh)
```

Or skip the installer: `install -Dm0755 candi ~/.local/bin/candi`.

### cargo install

```bash
cargo install --git https://github.com/mehboobulqadri/candi.git --tag v0.1.1 --locked --bin candi
```

Requires Rust stable and `clang` — MuPDF's bundled C sources compile at build
time (statically linked; not a runtime dependency).

### From source

```bash
git clone https://github.com/mehboobulqadri/candi
cd candi
cargo install --path crates/candi-gui --locked
```

If `~/.local/bin` is not on your `PATH`, add it, then run `candi book.pdf`.

AUR package planned (deferred until multi-OS packaging); `packaging/` already
contains a validated PKGBUILD.

## Roadmap

Versioned releases; feature work merges to `dev`, `dev` merges to `main` for major
releases. Full rules and details: [docs/roadmap.md](docs/roadmap.md).

- **v0.1** — shipped base + UI fixes: GUI reader, dual backends, themes,
  bookmarks, custom themes & keybinds.
- **v0.2** — EPUB & other formats, password-locked PDFs, optimizations/cleanup.
- **v0.3** — multi-OS (Windows, other Linux distros), deployments.
- **v0.4** — GitHub documentation site + cleaning.
- **v0.5** — Debian packaging + CI (AUR deferred until after multi-OS).
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
