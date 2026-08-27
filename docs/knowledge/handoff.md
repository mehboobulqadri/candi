# Handoff — next agent starts here

## State (2026-08-27)

Branch `slice/02-01-gui-reader`, tip `b06ff39`, **8 commits ahead of
origin/slice/02-01-gui-reader (`3866fa6`) — all local, unpushed** (push is
pending the user's dogfood/verify outcome per flow). Gates green: fmt,
clippy `-D warnings`, workspace tests with
`PDFIUM_LIB=~/.cache/candi-pdfium/chromium-7543` (= DIRECTORY).

The v0.1.0 candidate is complete: feature-complete GUI reader + polish +
editable config (keybinds, themes, prefs) + corrected docs + roadmap +
AUR packaging + real-book benchmarks. Independent review round-2 verdict
FIX-FIRST on 5 docs/regression items → **all fixed in `b06ff39`**;
targeted re-review was running at close-out — **verdict PENDING** (gates
green regardless; check the final reviewer message before acting on it).

### Commit map (tip → base)

- `b06ff39` fix(gui,docs): review round-2 — truthful theme-validation
  paragraph; README deps rust+clang (no mupdf); zoom_in physical combo
  parity (Shift+=/Ctrl+= variants enumerated); keybinds `schema_version`
  added+tolerated; empty-binding-list warn+defaults; prefs
  malformed-recents warn.
- `04ca989` chore(packaging): AUR PKGBUILD (candi 0.1.0-1,
  AGPL-3.0-only, clang makedepends, NO system-mupdf dep — mupdf-sys
  bundles static MuPDF), `.SRCINFO` skip-sums template, `AUR.md` guide,
  desktop file + icon vendored. Local `makepkg -f` validated end-to-end.
  `options=('!lto')` REQUIRED else makepkg's injected `-flto` breaks the
  bundled-C link.
- `b9dabf6` docs: architecture.md six stale sections corrected against
  code (trait block, sidecar dual v1-TUI/v3-GUI documented FROM
  `serialize_session`, themes section real, Spike 2/3 resolved); README
  rebuilt; `docs/roadmap.md` created (versioning rules + v0.1–v0.6);
  mockups moved UNRENAMED to `docs/design/mockups/`; dependabot PR #1
  CLOSED w/ comment (manual consolidated checkout bump planned
  post-merge).
- `a7eae11` feat(gui): `keybinds.json` (`$XDG_CONFIG_HOME/candi/
  keybinds.json`, seeded defaults, 17 actions, exact-modifier matching,
  tolerant per-entry fallback, conflict last-wins) + custom themes
  (`<config>/themes/*.yaml`, name==stem, Save-as in editor, picker
  customs w/ delete + Dark fallback; `themes_dir()` single source).
- `254c5b2` feat(gui,theme): nested YAML theme editor schema (parse
  accepts flat+nested; unknown groups preserved); ctrl±/zoom_delta
  editor font zoom 8..40; DEFAULT_THEME=Dark; prefs.toml
  (`config.toml` schema v1, `[appearance]` + `[[recents]]`,
  atomic+tolerant); recents via `record_open` choke point, cap10
  dedupe; accent seepage retint luma-gated (≥24 gap).
- `8944eea` fix(gui): anchor-preserving zoom/flow/resize/pinch
  (`center_anchor(page,frac)` machinery); slider+keys+pinch+fit flows
  anchored; flow switch snaps anchor to dest row-first page; resize
  keeps primary page.
- `0af4e49` feat(gui): chrome/sidebar polish (15 items) — distinct
  lucide icons for the duplicate fullscreen buttons; slider
  `trailing_fill` global fix (detached-handle illusion); page pill
  centered via screen-anchored `Area`; TOC rows via exact-rect
  `ui.put`; separators off; PgUp/PgDn labeled with WORDS (Inter lacks
  ←→ glyphs); TOC accent deepest-active-only w/ end-range computation +
  tests; min/max/close buttons removed (drag + dbl-click maximize kept);
  search field moved into sidebar; panel resizable 230–400 w/ collapse
  toggle + Ctrl+B; Ctrl+K opens keybinds; centered popups;
  `CANDI_UI_DEBUG=1` capture hook; `widgets.open.weak_bg_fill` themed.
- `a213302` fix(core): search char-boundary panic — scan_one_page byte
  index sliced mid-codepoint on accented text (é etc.), panicked the UI
  thread; fixed with `as_bytes()` compare + `is_char_boundary` guard;
  regression test café/résumé with FakeDoc.

## What awaits the user (stop-gates — explicit authorization required)

1. **Dogfood the build** — launch `candi book.pdf`, exercise reading,
   search, TOC, bookmarks, themes/keybinds/prefs edits across restarts.
   Specifically test **pinch-zoom on real hardware** (touchpad pinch
   gesture driving zoom_delta anchoring).
2. **Known deferred item**: highlighting the CURRENT match differently
   from other matches is NOT done (all matches same style).
3. **Tag MUST be literally `v0.1.0`** — the PKGBUILD URL/pkgver depends
   on the literal string. Then push, PR `slice/02-01-gui-reader` →
   `dev`, and `dev` → `main`.
4. **PR sequencing + versioning rules** live in `docs/roadmap.md`:
   majors are `v0.x` tags landing on main; post-release fixes are
   `v0.x.y`; features go to `dev`; new feature branches start AFTER each
   version cut. Timeline: v0.1 base+polish (HERE) → v0.2 epub+password
   PDFs+optimizations → v0.3 Windows+other Linux+deployments → v0.4
   GitHub docs/cleaning → v0.5 AUR/Debian+CI (AUR pulled EARLY — package
   ready in `packaging/`) → v0.6 Android beta.
5. **AUR submission steps** when authorized: `packaging/AUR.md`
   (account, dedicated ssh key `aur@`, clone
   `ssh://aur@aur.archlinux.org/candi.git`, master-only, updpkgsums +
   printsrcinfo each release).

## Flags / infra state

- **`dev` and `main` are UNPROTECTED** — user protection decision pending.
- `origin/slice/docs-catchup` is contained in `dev` — prune candidate,
  needs user sign-off.
- Branch is 8 commits unpushed (awaiting user-verify outcome).
- Cargo.lock conflicts expected if a stale cargo dependabot PR revives
  (none open now; dependabot PR #1 closed).
- **`/tmp` tmpfs (7.7G) overflows on Rust debug/test target dirs → use
  `/mnt/personal/tmp`** for large builds/tests (SIGBUS incident hit two
  sessions running). opencode disk cleanup still deferred
  (`~/.local/share/opencode/opencode.db` ≈ 26G).

## Benchmarks (real books, `/mnt/personal/Books` — 7 unique, 0.8–50 MB, 148–1556 pp)

- Core `reader_peak` avg: mupdf 17.6 MB / pdfium 18.4 MB; worst case
  58 MB (silberschatz, pdfium) → 200 MB gate passes with ~3.4× headroom.
- `full_pass` hazards (RECORDED, NOT GATED): sdi2 pdfium 1996 MB
  (image-heavy full sweep), silberschatz mupdf 295 MB (MuPDF TLS store,
  known). Gate only `reader_peak`.
- GUI process RSS: ~190 MB avg mupdf / ~213 MB pdfium; egui floor
  ~170 MB — so GUI floor dominates the core budget.
- Open + first render avg: 61 ms mupdf / 23 ms pdfium.

## Install (still valid)

- `~/.local/bin/candi` → symlink to `target/release/candi` (stale
  `~/.cargo/bin/candi` removed — it shadowed the symlink once).
- Desktop entry `~/.local/share/applications/candi.desktop`
  (Icon=candi, MimeType=application/pdf, StartupWMClass=candi).
- NOTE: the symlink points into `target/release` — a `cargo clean`
  invalidates it until the next build.

## Capture discipline (do not regress)

`scripts/shot.sh` matches the spawned window BY PID; it internally
pkills by path regex — confirm no foreign `candi` instance first
(never `pkill -f`, it self-matches). Backgrounded GUI launches must
detach stdio: `nohup … < /dev/null & disown` — an inherited stdout FD
hangs the driving tool. Verify WHAT you captured; the user works on
this machine simultaneously — ask for quiet moments for final evidence.
Display scale factor makes logical ≠ physical sizes in captures.

## Pointers

- egui 0.30 gotchas and process lessons: `.agents/meta/memory.md`
  (entries 2026-08-25 onward — most recent includes the polish-batch
  layout lessons).
- Session-by-session decisions/issues: `docs/knowledge/log.md`.
