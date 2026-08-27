# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal document reader (GUI text) on a shared Rust core; dual PDF
backends (MuPDF default, PDFium permissive). TUI (`candi-tui`) is internal.

## Current focus

**v0.1.0 SHIPPED** — tag `v0.1.0` on `main` (`8995b64`); `dev` at `f864a15`
(PR #21). Next: repo visibility → public (user) → AUR push (blocked until
public); then post-release fixes `v0.1.1` on `dev` (backlog in
`docs/knowledge/handoff.md`).

## Active constraints

- **Versioning:** post-release fixes `v0.1.x` on `dev`; features only after
  the v0.1.1 cut; tags land on `main` (`docs/roadmap.md`).
- **AUR blocker:** repo PRIVATE → PKGBUILD tag-tarball URL 404s; flip public first.
- **Entry:** `candi [book.pdf] [--backend …]`; hooks `CANDI_NO_GUI=1`, `CANDI_UI_DEBUG=1`; config in `$XDG_CONFIG_HOME/candi/`.
- Tests/benches: `PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` (DIRECTORY);
  big target dirs on `/mnt/personal/tmp`, never `/tmp` tmpfs.
- **`creds.yml` is NEVER read** (gitignored secrets; use git grep/excludes).
- RSS gate: `reader_peak` only (full_pass recorded, not gated).

## Stack

Rust 1.97.1, MuPDF 0.8.0, PDFium `pdfium-render 0.8.37`, ratatui 0.30.2,
egui 0.30, serde_json/serde_yaml, GitHub Actions + cargo-deny.
