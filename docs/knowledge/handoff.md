# Handoff — next agent starts here

## State (2026-08-28)

**v0.1.2 IS CUT and validated.** PR #30 (dev→main) merged; merge
`37362d9` = tag `v0.1.2`; `dev` == `main` == `37362d9`. Release live:
tar.gz + zip + sha256s all verified via `sha256sum -c`; install.sh run
per README verbatim; smoke `CANDI_NO_GUI=1 candi --help` OK. Local
`~/.local/bin/candi` is now the INSTALLED RELEASE BINARY, not the dev
symlink — a dev build replaces it only on the next in-repo release
build. 8 rounded RGBA icons + `candi.desktop` installed to `~/.local`.

User feedback on v0.1.2: keybinds verified working (+/- page turns,
Ctrl+- app zoom; Shift+- also pages — accepted). Full cut record incl.
root causes (chrome-focus sweep, recolor LRU): `log.md` 2026-08-28
(v0.1.2 cut + final wave). The v0.1.2-RC fix-wave + repo-hardening
details live in `log.md` 2026-08-28 entries — not repeated here.

## v0.1.3 scope (IN PROGRESS)

1. **Trackpad pinch-to-zoom** via `zwp_pointer_gestures_v1` side-channel
   — IN FLIGHT, user finger-test pending. winit 0.30.13 emits
   PinchGesture only on macOS/iOS; no Wayland binding (Hyprland serves
   the protocol; egui-winit's PinchGesture→Zoom is dead code on Linux).
   Fix = raw-window-handle + `wayland-client` `from_external_display`
   (smithay-clipboard pattern), thread-owned queue → mpsc →
   `set_zoom_percent`; Chromium ozone `wayland_zwp_pointer_gestures` is
   the reference pattern (absolute scale → per-update scale_now/prev
   multipliers, cancelled-end no-op). Debug hook: `CANDI_GESTURE_DEBUG=1`.
   Shortcuts copy flips from "not supported" ONLY once proven. Research:
   `log.md` 2026-08-28 pinch entry.
2. **Scroll sensitivity** increase + settings slider for trackpad scroll
   amount — queued.
3. **Smooth page-turn transitions** — queued.

Then cut v0.1.3 via the standard PR path: PR dev→main (ruleset
`main-protection` 21686764: PR-only, 0 approvals, 4 required checks, no
bypass; `dev-protection` 21686769: force/delete blocked) → tag LITERAL
`v0.1.3` on the main merge commit → release.yml (tar.gz+zip+sha256s) →
merge main back into dev → release validation (download tar.gz AND zip,
`sha256sum -c`, install.sh per README).

## Standing constraints (do not regress)

- **Gates BEFORE push — no exceptions**, even under cancellation pressure
  (incident `0314ede`: pushed red → CI red on dev; log.md 2026-08-28).
- **Versioning:** v0.1.x fixes on `dev`; tags land on `main`; merge
  `main`→`dev` after EVERY cut (`docs/roadmap.md`).
- **`creds.yml` is NEVER read** — gitignored personal secrets; use
  `git grep` / explicit excludes.
- Tests/benches: `PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` (a
  DIRECTORY); big target dirs on `/mnt/personal/tmp`, never `/tmp`
  tmpfs; headless runs that write sidecars COPY fixtures to tmp first.
- RSS gate: `reader_peak` only (full_pass recorded, not gated).
- Entry/hooks: `candi [book.pdf] [--backend …]`; `CANDI_NO_GUI=1`,
  `CANDI_UI_DEBUG=1`, `CANDI_GESTURE_DEBUG=1` (v0.1.3 pinch); config in
  `$XDG_CONFIG_HOME/candi/`; NO `--version` flag.
- Machine: opencode.db+WAL ~44G cleanup PENDING (user runs — steps
  below); big parallel builds OOM the box.

## Machine cleanup (user runs when ready — still pending)

`~/.local/share/opencode/opencode.db` ~27G + WAL ~17G. Stop opencode →
`sqlite3 ~/.local/share/opencode/opencode.db 'PRAGMA
wal_checkpoint(TRUNCATE); VACUUM;'` → if space-blocked (VACUUM needs
~2×), `rm` db+wal+shm after closing → restart. Expects ~40G back.

## Backlog

- rotated-page dest_top parity + fixture (PDFium /Rotate vs MuPDF
  post-rotation space)
- welcome_block_height measured-layout refactor (magic-number twin)
- tinted-chrome per-frame caching
- per-bookmark page-load caching at outline build
- search hit cap (10k + truncated note); search snippet double-extract
- Ctrl+Q save-failure visibility; dialog-panic feedback line
- highlight unwrap_or_default caching; cache.insert oversized-bool
- canvas_bg custom-theme degeneration doc note
- RSS profiling pass (idle ~190MB = egui floor ~170MB + app;
  heaptrack/massif, v0.2 roadmap slot)
- parity test OPEN_TIMEOUT 5s cold-start flake
- progress.md merge-history rows 01/06–01/08 gap (pre-existing)
- headless `CANDI_NO_GUI` hook fails on GUI-written v3 session sidecars
  ("unsupported sidecar schema version 3") — hook should tolerate session
  sidecars (pre-existing since v0.1.0, GUI unaffected)
- crates.io publish deferred — path-dep chain (core/pdf/theme/cli/gui)
  must publish in dependency order + user crates.io token
- no `--version` flag (clap disabled) — consider enabling for support
  triage
- Node 20 deprecation warnings on actions/checkout@v4 — cosmetic
- re-enable dependabot later (user decision) + prune its 6 lingering
  remote branches first
- AUR post-multi-OS (packaging/ ready; day-of steps in
  `packaging/AUR.md`)
- README welcome screenshot retake (optional)

## Capture discipline (do not regress)

- `scripts/shot.sh` matches by PID; `pkill -x candi`, never `-f`;
  backgrounded GUI launch: `nohup … < /dev/null & disown`.
- Headless runs that write sidecars COPY fixtures to tmp first.
- **Hyprland here runs the ML4W Lua dispatcher shim** — plain
  `hyprctl dispatch fullscreen` is broken Lua; use
  `hyprctl dispatch 'hl.dsp.window.fullscreen()'`; verify focus via
  `activewindow` pid.
- **Reader theme comes from the SESSION SIDECAR** (`{pdf}.candi.toml`
  `[appearance] theme`), NOT `config.toml` — screenshot pre-seeding must
  seed both.
- **grim region capture: contain launch→capture→kill in ONE shell call**
  or the window loses focus/top-of-stack.

## Pointers

- egui 0.30 gotchas + process lessons: `.agents/meta/memory.md`
  (2026-08-25 → 2026-08-28 entries).
- Session decisions/incidents: `docs/knowledge/log.md` (2026-08-28
  entries: v0.1.2 cut + final wave, pinch research, v0.1.2-RC wave,
  repo hardening, incidents, RAM/swap + disk cleanup).
- AUR day-of steps: `packaging/AUR.md`.
- Benchmarks (real books, egui floor ~170MB): `log.md` 2026-08-27.
- Versioning rules + roadmap: `docs/roadmap.md`.
