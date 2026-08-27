# Project status

Always-on context. Loaded automatically at session start — keep it SMALL (under ~30 lines).

## Project

Candi — minimal document reader (TUI + GUI text) on a shared Rust core; dual PDF
backends (MuPDF default, PDFium permissive).

## Current focus

**v0.1.0 candidate complete** on `slice/02-01-gui-reader` (HEAD `b06ff39`, 8 commits
local/unpushed): reader + polish + editable keybinds/themes/prefs + corrected docs +
`docs/roadmap.md` + AUR packaging + real-book benches (gate passes ~3.4× headroom).
Gates green; targeted re-review verdict pending. Next: user stop-gates — dogfood →
tag `v0.1.0` (literal; PKGBUILD URL) → PR to `dev` → `dev`→`main`.

## Active constraints

- **Stop-gates:** dogfood, tag, `main` merge — explicit user authorization required.
- **Versioning:** majors `v0.x` land on main; post-release fixes `v0.x.y`; features
  go to `dev`; new feature branches only AFTER each cut (`docs/roadmap.md`).
- **Platforms:** Linux dogfood; Windows CI deferred (Linux-only matrix).
- **Entry points:** `candi-tui book.pdf`; `candi [book.pdf] [--backend …]`.
- **Hooks/env:** `candi-cli` internal lib; `CANDI_NO_GUI=1` test hook;
  `CANDI_UI_DEBUG=1` ui-capture hook; config in `$XDG_CONFIG_HOME/candi/`.
- **RSS budget:** gate **`reader_peak`** only (full_pass recorded, not gated).
- **Build hygiene:** big debug/test target dirs belong on `/mnt/personal/tmp`, not
  `/tmp` tmpfs.

## Stack

Rust 1.97.1, MuPDF 0.8.0, PDFium `pdfium-render 0.8.37`, ratatui 0.30.2, egui 0.30,
serde_json/serde_yaml, GitHub Actions + cargo-deny.
