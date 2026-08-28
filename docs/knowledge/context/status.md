# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal document reader (GUI text) on a shared Rust core; dual PDF
backends (MuPDF default, PDFium permissive). TUI (`candi-tui`) is internal.

## Current focus

**v0.1.3 IN PROGRESS.** v0.1.2 IS CUT and validated: PR #30 dev→main
merged, merge `37362d9` = tag `v0.1.2`; `dev` == `main` == `37362d9`;
release live (assets sha256-verified, install.sh run — local
`~/.local/bin/candi` is now the release binary). User confirmed keybinds
working. v0.1.3 scope: (a) trackpad pinch-to-zoom via
`zwp_pointer_gestures_v1` side-channel — in flight, finger-test pending,
`CANDI_GESTURE_DEBUG=1`; (b) scroll sensitivity + settings slider;
(c) smooth page-turn transitions. Then standard cut path (ruleset
21686764, tag literal `v0.1.3`, merge main→dev, release validation).

## Active constraints

- **Versioning:** v0.1.x fixes on `dev`; tags land on `main`; merge
  `main`→`dev` after EVERY cut (`docs/roadmap.md`).
- **Gates BEFORE push** — no exceptions, even under cancellation pressure
  (CI-red-on-dev incident 0314ede).
- **Private repo:** release URL + tag-tarball 404 anonymously until flip.
- **Entry:** `candi [book.pdf] [--backend …]`; hooks `CANDI_NO_GUI=1`,
  `CANDI_UI_DEBUG=1`; config in `$XDG_CONFIG_HOME/candi/`; NO `--version` flag.
- Tests/benches: `PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` (DIRECTORY);
  big target dirs on `/mnt/personal/tmp`, never `/tmp` tmpfs; headless runs
  that write sidecars COPY fixtures to tmp first.
- **`creds.yml` is NEVER read** (gitignored secrets; use git grep/excludes).
- RSS gate: `reader_peak` only (full_pass recorded, not gated).
- Machine: opencode.db+WAL ~44G cleanup PENDING (user runs post-session —
  steps in handoff); big parallel builds OOM the box.

## Stack

Rust 1.97.1, MuPDF 0.8.0, PDFium `pdfium-render 0.8.37`, ratatui 0.30.2,
egui 0.30, serde_json/serde_yaml, GitHub Actions + cargo-deny.
