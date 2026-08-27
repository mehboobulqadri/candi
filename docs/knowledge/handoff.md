# Handoff — next agent starts here

## State (2026-08-28)

**v0.1.0 SHIPPED.** Annotated tag `v0.1.0` → `8995b64` on `main`; `dev` at
`f864a15` (PR #21 merge from `slice/02-01-gui-reader`). Gates green
throughout: fmt, clippy `-D warnings`, workspace tests with
`PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` (= DIRECTORY);
`CARGO_TARGET_DIR` under `/mnt/personal/tmp`.

Release tail on the slice (top → base):

- `ce4486b` icon branding — rounded 21%-radius multi-res hicolor set
  (16–512) + embedded 256px window icon (`image 0.25`, png-only).
- `8960d1a` pre-tag hardening (review battery) — outline depth cap 64
  parity, corrupt-sidecar save-guard + banner, failed-open-over preserves
  sidebar, search `catch_unwind`, theme-delete keeps entry on failure,
  PDFium missing-render-symbol → error, `dest_top` is_finite guards, FIFO
  preflight `is_file`, PoisonError recovery, theme YAML 256 KiB cap.
- `754d73d` GUI-only public story — README/roadmap/PKGBUILD/.SRCINFO/AUR.md
  de-advertise `candi-tui`.
- `c507c15` dogfood behavioral batch (19 issues) — TextEdit FocusGuard;
  keybinds `DEFAULTS_SCHEMA_VERSION` 2 migration healing stale v1 seeds;
  render FailState ledger w/ 250ms→1s→4s backoff + click-to-retry +
  stale-scale pruning; TOC `dest_top` end-to-end (MuPDF XYZ top / PDFium
  `FPDFDest_GetLocationInPage` + y-flip) + accent same-page semantics +
  `toc_follow` clicked-row preference; bottom-center page toast 700ms/200ms;
  async search worker (`SearchSession::step` + mpsc + AtomicBool);
  non-blocking rfd dialog single-flight; startup goes straight to welcome
  (`pick_pdf` removed).
- Earlier candidate commits (reader, polish, config, docs, packaging):
  full commit map in `log.md` 2026-08-27 entry.

Review battery all PASS (fixes gate-verified): independent round-2 worktree
review PASS; security FIX-THEN-PASS (1 MED crash-DoS outline recursion →
depth cap; LOWs fixed or deferred); silent-failure-hunter PASS (H1
data-loss + 4 MEDs → fixed).

## What awaits the user

1. **BLOCKER — repo visibility.** Repo is PRIVATE → the AUR PKGBUILD
   source URL (tag tarball) 404s anonymously. Flip visibility to public
   BEFORE the AUR push.
2. **AUR push.** One-time prep already given: account, ssh pubkey upload,
   `~/.ssh/config` `Host aur.archlinux.org` (`User aur`, `IdentityFile
   ~/.ssh/aur`, `IdentitiesOnly`), `ssh-keyscan` → known_hosts, ssh test.
   Day-of: clone `ssh://aur@aur.archlinux.org/candi.git`, cp
   `packaging/{PKGBUILD,.SRCINFO}`, `updpkgsums`,
   `makepkg --printsrcinfo > .SRCINFO`, commit `candi 0.1.0`, push
   `master` ONLY. Full guide: `packaging/AUR.md`.
3. **Pending decisions:** branch protection on `dev`+`main` (both
   UNPROTECTED); prune `origin/slice/docs-catchup` (merged); stale
   `origin/master` ref (trivia, resurfaced from an old handoff).
4. **Versioning:** post-release fixes = `v0.1.1` on `dev`; features start
   only AFTER the v0.1.1 cut, per `docs/roadmap.md` (v0.2 epub+password
   PDFs+optimizations → v0.3 Windows/other Linux → v0.4 GitHub docs →
   v0.5 AUR/Debian+CI → v0.6 Android beta).

## v0.1.1 backlog (verbatim — commit trailers + reviews)

- rotated-page dest_top parity + fixture (PDFium /Rotate vs MuPDF
  post-rotation space)
- welcome_block_height measured-layout refactor (magic-number twin)
- tinted-chrome per-frame caching
- per-bookmark page-load caching at outline build
- search hit cap (10k + truncated note)
- search snippet double-extract
- Ctrl+Q save-failure visibility
- dialog-panic feedback line
- highlight unwrap_or_default caching
- cache.insert oversized-bool
- canvas_bg custom-theme degeneration doc note
- Wayland pinch support (zwp_pointer_gestures side-channel — winit has no
  Linux pointer-gesture support; Ctrl+scroll works, shortcuts window
  notes this)
- RSS profiling pass (idle ~190MB = egui floor ~170MB + app;
  heaptrack/massif, v0.2 roadmap slot)
- parity test OPEN_TIMEOUT 5s cold-start flake
- progress.md merge-history rows 01/06–01/08 gap (pre-existing)

## Flags / infra state

- `PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` (= DIRECTORY) required
  for workspace tests/benches.
- Big debug/test target dirs on `/mnt/personal/tmp`, never `/tmp` tmpfs
  (7.7G overflow, SIGBUS hit two sessions).
- **`creds.yml` is NEVER read** — gitignored personal secrets file; no
  cat/grep/recursive scan touches it (use `git grep` / explicit excludes).
- `dev`/`main` UNPROTECTED; `origin/slice/docs-catchup` prune candidate;
  stale `origin/master` ref — user decisions pending.
- Cargo.lock conflicts expected if a stale cargo dependabot PR revives
  (none open; dependabot PR #1 closed).

## Install (carried forward — unchanged)

- `~/.local/bin/candi` → symlink to `target/release/candi` (a stale
  `~/.cargo/bin/candi` once shadowed it). The symlink points into
  `target/release` — a `cargo clean` invalidates it until the next build.
- Desktop entry `~/.local/share/applications/candi.desktop`
  (Icon=candi, MimeType=application/pdf, StartupWMClass=candi).

## Capture discipline (do not regress)

`scripts/shot.sh` matches the spawned window BY PID; it pkills by path
regex — confirm no foreign `candi` instance first, and **`pkill -x candi`,
never `pkill -f`** (self-matches). Backgrounded GUI launches must detach
stdio: `nohup … < /dev/null & disown` — an inherited stdout FD hangs the
driving tool. Verify WHAT you captured; the user works on this machine
simultaneously — ask for quiet moments for final evidence. Display scale
factor makes logical ≠ physical sizes in captures.

## Pointers

- egui 0.30 gotchas + process lessons: `.agents/meta/memory.md`
  (2026-08-25 → 2026-08-28 entries).
- Session decisions/issues: `docs/knowledge/log.md` (2026-08-28 entries
  cover the release: GUI-only pivot + SRCINFO incident, icon branding,
  dogfood batch, review battery, main/dev divergence reconciliation,
  v0.1.0 cut + private-repo blocker).
- Benchmarks (real books: `reader_peak` gate vs `full_pass` recorded-not-
  gated, egui floor ~170 MB): recorded in `log.md` 2026-08-27.
- Versioning rules + roadmap: `docs/roadmap.md`.
