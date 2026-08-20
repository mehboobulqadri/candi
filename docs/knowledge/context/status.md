# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).
Update only when focus or constraints actually change.

## Project

Candi — minimal, fast cross-platform document reader. One native Rust core; TUI now, GUI later. AGPL-3.0. Dual switchable PDF backend (MuPDF default + PDFium) behind one trait; Poppler rejected (spike-1).

## Current focus

Base merged: slice/00-base (1d667ef) → dev (07d02a8) → main (414252d), progress.md (e519732). Next session starts Phase 00: slice 00/01-workspace-bootstrap.

## Active constraints

- User-mandated 4-stage quality drill + slice workflow (see .agents skills; only an explicit user brief lifts them).
- Verify locally before any merge/deploy — local green = CI green; never do something that can fail.
- No web-publishing of implementation/spikes/knowledge; GH Pages deferred.
- Never commit identity: creds.yml gitignored, docs carry {{placeholders}}, filled via scripts/sync-creds.sh.
- Never prune skills without explicit user confirmation.
- Remote never pushed yet; first push needs auth setup.

## Stack

Rust 1.97.1 (rust-toolchain.toml pinned). Workspace crates candi-{core,pdf,theme,tui,cli}. ratatui/crossterm (TUI). mupdf 0.8.0, pdfium-render 0.8.37. CI: fmt, clippy -D warnings, test, docs-links.
