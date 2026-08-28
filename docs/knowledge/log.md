# Log

Append-only project history: decisions made, issues faced, root causes.
Never edit entries in place — append below the marker with a date header.

Format per entry:

## YYYY-MM-DD

- **Decision/Issue:** what happened.
- **Why:** the reasoning or root cause.
- **Changed:** what changed because of it.

<!-- entries below -->

## 2026-08-20

- **Decision/Issue:** Base commit merged — slice/00-base (1d667ef) → dev (07d02a8) → main (414252d); progress.md updated (e519732).
- **Why:** Reviewed base (docs + CI + license + skills provenance) passed both review rounds; repo now ready for Phase 00 implementation.
- **Changed:** dev and main carry the reviewed base; next session starts slice 00/01-workspace-bootstrap.

- **Decision/Issue:** Skill-pruning mistake — 27 skills wrongly deleted during a "token optimization" pass that was intended to be docs-only; 23 restored from the mia template repo, 4 permanently dropped (agent-browser, e2e-testing, frontend-design, vercel-react-best-practices — web-only, no web surface in this project).
- **Why:** The pass overreached its stated scope; user never confirmed pruning.
- **Changed:** .agents/skills/ restored to 35; memory.md, registry.md, skills-lock.json corrected. Lesson: never prune skills without explicit user confirmation.

- **Decision/Issue:** Creds/identity system — gitignored creds.yml (name/email/github/project/license) fills {{placeholders}} in docs via scripts/sync-creds.sh (escaped sed, post-condition fails on leftover placeholders, skips placeholder-free files); skills-lock.json un-ignored and committed.
- **Why:** Identity must never be committed; skills-lock was gitignored, so provenance was at risk.
- **Changed:** .gitignore updated, sync-creds.sh added, skills-lock.json now tracked.

- **Decision/Issue:** Review round on base — 6 findings: M1 Phase-1 gate-status contradiction, N2 sed escaping, N3 AGPL-3.0 vs -or-later drift, N4 page-offset contradiction, N5 frankl fixture wording, N6 dead _config.yml entries.
- **Why:** Independent reviewer pass (FIX-THEN-MERGE) on the base.
- **Changed:** All findings fixed, amended into 1d667ef.

- **Decision/Issue:** Merge topology — amending the base commit produced a NEW orphan root with no common ancestor with the pre-review root f4305ed; merged with --allow-unrelated-histories -X theirs; removed stale docs/.github/workflows/pages.yml (pre-review artifact, dropped by the reviewed base).
- **Why:** `git commit --amend` on a root commit replaces it with a new root commit, severing parentage.
- **Changed:** dev/main first-parent points at f4305ed; end tree byte-identical to the reviewed base. Cosmetic artifact; regraft later if desired.

- **Decision/Issue:** Dual PDF backend chosen — MuPDF (default) + PDFium behind one trait; Poppler rejected.
- **Why:** Spike-1 evidence (spikes/results/spike-1-pdf-backend.md): MuPDF fits the minimal TUI target; Poppler licensing/build overhead; PDFium covers MuPDF gaps. Final selection deferred to the user later in v0.1.
- **Changed:** Architecture and candi-pdf crate plan codify the backend trait.

- **Decision/Issue:** License AGPL-3.0 chosen with a permissive escape hatch noted for downstream parts.
- **Why:** User preference; AGPL matches the open-source posture while the escape hatch keeps licensing flexible.
- **Changed:** LICENSE, SECURITY docs; SPDX AGPL-3.0 headers planned for 01-workspace-bootstrap.

- **Decision/Issue:** User-mandated 4-stage quality drill + slice workflow (slice = one independent commit on slice/NN-name off dev, two review rounds, merge to dev, dev→main after major phases, progress.md updated before every merge) and local-verification-first rule.
- **Why:** Deterministic quality gate; local green = CI green; never do something that can fail.
- **Changed:** Codified in slice-workflow/dev-loop skills; governs all future implementation work.

## 2026-08-21 (docs catch-up)

- **Decision/Issue:** Documentation still described pre-implementation / CLI-only state after PRs #17–#18 landed TUI polish and Linux GUI on `dev`.
- **Why:** README, progress, architecture, and knowledge context were not updated when user binaries shifted to `candi-tui` + `candi` and `candi-cli` became a lib.
- **Changed:** README quick start; progress rows for 01/11, tui-readability, gui-text-linux; architecture diagram/frontends; handoff + status; 01-v0.1 README one-liner on compressed GUI scope. No tag/`main` claims.

## 2026-08-21 (01/11 close-out)

- **Decision/Issue:** Slice 01/11 v01-release engineering merged to `dev` via PR #16 (slice feat `d7ce832`, HEAD `74a241e`, merge `638f8bcbf97b7c89e76ff38778ab64e66c724432`, mergedAt 2026-08-20T21:48:06Z). Independent reviewer APPROVE. CI `32420211385` all green including Windows rust-checks.
- **Why:** Phase 01 final engineering slice — release checklist, Windows CI matrix, version 0.1.0 prep, honest acceptance posture.
- **Changed:** Windows in rust-checks matrix (ubuntu + windows × default + pdfium-only); pdfium-win-x64 `chromium/7543` (dll sha256 `6b963c2be9cacbaa0c0c7f4bf6d20d2fd16729ebdaa9989978b0f7b119c1c1cb`). MuPDF builds on Windows. Linux-only apt fontconfig + bench (`/proc` RSS). Windows clippy fixes: cfg-split `FPDF_GetLastError`, unix-gated `OpenOptions`, needless_return in parity skip. `acceptance.md` honest: dogfood not met; tag/`main` not done; real-books bench SKIPPED. Phase 01 **code** complete on `dev`. Handoff shifts to user stop-gates only.

- **Decision/Issue:** Windows platform clippy only surfaced after Windows matrix added in 01/11.
- **Why:** Ubuntu-only CI missed Windows-specific cfg and cast issues (`c_ulong` is u32 on Windows — unnecessary `as u32`).
- **Changed:** Windows jobs in rust-checks; memory lesson on earlier matrix coverage.

## 2026-08-21 (01/10 close-out)

- **Decision/Issue:** Slice 01/10 candi-cli merged to `dev` via PR #15 (slice feat `de1ecce`, fix `5089cc6`, merge `9b86661f48053ca8b27ff31ed9b32b97536df826`, mergedAt 2026-08-20T19:59:31Z).
- **Why:** Phase 01 tenth slice — CLI entry wiring factory open, sidecar resume, TUI launch, and explicit error UX.
- **Changed:** Binary `candi`; clap FILE + `--backend` (default mupdf); `Args::try_parse()` — all parse failures exit 1 (including usage/unknown flags). Open → sidecar load → resume via `ViewState::goto_page`; corrupt sidecar warns; `UnsupportedSchema` fail loud. `CANDI_NO_TUI=1` prints `page=<1-based>`, still saves sidecar. `candi_tui::run(doc, filename, initial) -> Result<ViewState, RunError>`. 10 CLI integration tests. Handoff advances to 01/11 engineering; tag/dogfood/main-merge stop-gates need user.

- **Decision/Issue:** Headless CI for CLI integration tests — TUI startup not suitable for CI matrix.
- **Why:** TUI requires terminal; TestBackend covers rendering but not full binary spawn path.
- **Changed:** `CANDI_NO_TUI=1` env hook skips TUI, prints current page, persists sidecar — enables spawn-based integration tests without raw mode.

- **Decision/Issue:** clap parse failures pinned to exit 1 (same as runtime errors).
- **Why:** v0.1 accepts uniform exit code with distinct stderr text per failure kind — simpler than per-kind code table.
- **Changed:** `Args::try_parse()` path exits 1 for usage, unknown flags, and missing args; hazard added to handoff.

## 2026-08-20 (01/09 close-out)

- **Decision/Issue:** Slice 01/09 candi-tui merged to `dev` via PR #14 (slice feat `238d410`, TerminalGuard `8ae4cc4`, deny Zlib `6f3c8c8`, merge `62739c99305d041620090e3fb7a1556b2f9c51da`, mergedAt 2026-08-20T19:32:23Z). Independent reviewer APPROVE after REQUEST CHANGES (raw-mode).
- **Why:** Phase 01 ninth slice — minimal TUI shell over shared core with honest test posture and terminal safety.
- **Changed:** `candi-tui` crate: ratatui 0.30.2, crossterm 0.29.0; §6 key bindings; `TestBackend`; Spike 2 frankl SKIPPED honest; `TerminalGuard` Drop RAII; `TERM=dumb` before raw mode; `candi-theme` dep dropped from tui. cargo-deny: foldhash Zlib allowed in `deny.toml`. Handoff advances to 01/10.

- **Decision/Issue:** Independent reviewer REQUEST CHANGES on raw-mode / terminal restore — tests could leave terminal in raw mode without RAII guard.
- **Why:** `enable_raw_mode` without guaranteed restore breaks subsequent test runs and CI honesty.
- **Changed:** `TerminalGuard` with Drop restore (`8ae4cc4`); `TERM=dumb` before raw mode in tests; hazard added to handoff.

- **Decision/Issue:** cargo-deny failed on foldhash/hashbrown Zlib license.
- **Why:** Transitive dependency license not in allow list.
- **Changed:** Zlib allowed in `deny.toml` (`6f3c8c8`); memory lesson on deny.toml maintenance.

## 2026-08-20 (01/08 close-out)

- **Decision/Issue:** Slice 01/08 search-abstraction merged to `dev` via PR #13 (slice feat `9c7e0f1`, wrap fix `a77a874`, progress `7f6655a`, merge `4af89e8c499da22a622fd20bb275472da51c8a06`, mergedAt 2026-08-20T18:55:41Z). Independent reviewer APPROVE after REQUEST CHANGES (wrap rescan). CI run `32404983245`.
- **Why:** Phase 01 eighth slice — shared `SearchSession` abstraction over both PDF backends for lazy, case-insensitive document search.
- **Changed:** `SearchSession`: page-at-a-time lazy scan; case-insensitive `to_lowercase`; results `(page, offset)` only; `start_page` + wrap; empty query → 0 `page_text` calls; errors propagate. Handoff advances to 01/09.

- **Decision/Issue:** Wrap scan with `start_page > 0` rescanned the start page — duplicate work and wrong termination.
- **Why:** Wrap loop did not stop at `start_page` when continuing from a mid-document start.
- **Changed:** Fix `a77a874`; test asserts `call_count == page_count`; hazard added to handoff; memory lesson on wrap boundary.

## 2026-08-20 (01/07 close-out)

- **Decision/Issue:** Slice 01/07 reading-position-sidecar merged to `dev` via PR #12 (slice feat `bd440c3`, progress `a82aaa0`, merge `decce477537a4196cc50f61065b3088f4b344ebd`, mergedAt verified via `gh pr view`). Independent reviewer APPROVE. CI run `32402904183`: all jobs success.
- **Why:** Phase 01 seventh slice — versioned reading-position sidecar with tolerant reads and atomic writes.
- **Changed:** `{pdf}.candi.toml` sidecar (schema v1); `Load::{Missing,Loaded,Corrupt}`; `Error::UnsupportedSchema` only for schema > 1; atomic temp+rename+unix dir fsync; PDF never modified; last-write-wins; handoff advances to 01/08.

- **Decision/Issue:** Sidecar read semantics — missing file is not corrupt file.
- **Why:** Fresh start vs recoverable warning must be distinguishable for TUI/GUI callers.
- **Changed:** `Load` enum separates `Missing` from `Corrupt`; only unsupported schema version (> 1) is a hard error.

- **Decision/Issue:** Reviewer nits carried forward (non-blocking): hand-rolled `unix_days_to_ymd` untested; Windows atomic test vacuous pass; `sync_dir` failure after successful rename still returns `Err`.
- **Why:** Edge-case clarity and cross-platform test honesty — not merge blockers.
- **Changed:** Nits table in handoff; optional cleanup when touching `state.rs` again.

## 2026-08-20 (01/06 close-out)

- **Decision/Issue:** Slice 01/06 candi-core-navigation merged to `dev` via PR #11 (slice feat `8b955fa`, progress `e44a58d`, merge `994a154`, mergedAt 2026-08-20T18:17:13Z). Independent reviewer APPROVE. CI run `32401474027`: all jobs success.
- **Why:** Phase 01 sixth slice — `ViewState` navigation model in `candi-core` (page + scroll clamping, no TUI coupling).
- **Changed:** `ViewState` in `candi-core`; 15 tests; handoff advances to 01/07; remote slice branch deleted; worktree `/tmp/candi-slice-01-06` removed.

- **Decision/Issue:** `ViewState` navigation semantics pinned for downstream slices.
- **Why:** TUI/GUI layers must share one core model without re-deriving clamp rules.
- **Changed:** 0-based page; clamp never `Result`/panic on realistic inputs; `max_scroll` caller-supplied (no TUI types); page change resets scroll to 0; empty doc stays page 0; `Copy`.

- **Decision/Issue:** Reviewer nits carried forward (non-blocking): `scroll_down` should use `saturating_add`; document inclusive `max_scroll`; add explicit first/last page scroll-reset tests.
- **Why:** Edge-case clarity and overflow safety — not merge blockers.
- **Changed:** Nits table in handoff; optional cleanup in a later touch of `candi-core`.

## 2026-08-20 (01/05 close-out)

- **Decision/Issue:** Slice 01/05 benchmarks-both-backends merged to `dev` via PR #10 (slice HEAD `883a98c`, gate-fix `1fb4242`, merge `fc5a2af`, mergedAt 2026-08-20T16:52:55Z). Independent reviewer APPROVE at `883a98c`. CI run `32393653573` on `883a98c`: all 7 jobs success.
- **Why:** Phase 01 fifth slice — production bench harness for both backends, corpus comparison against architecture.md §Cross-cutting budget, spike doc updates.
- **Changed:** `crates/candi-pdf/benches/bench.rs`, `bench/run.sh`, CI bench job with libpdfium; handoff advances to 01/06; remote slice branch deleted.

- **Decision/Issue:** Dual-peak RSS methodology — **`reader_peak`** (open + first page + search/nav page window) is the v0.1 **200 MB gate**; **`full_pass_peak`** after full page sweep is **printed, not gated**.
- **Why:** Full sweep hits MuPDF TLS `fz_context` decode store (`FZ_STORE_DEFAULT` ~256 MiB); dropping `Page` does not empty the store — MuPDF silberschatz ~43–44 MB reader vs ~295 MB full pass.
- **Changed:** Architecture table uses `reader_peak` for Peak RSS; `full_pass_peak` recorded not gated; CI `--fixtures-only`, `BENCH_CHECK_BUDGET=0`.

- **Decision/Issue:** Per-page `fz_empty_store` experiment **worsened** peak (~940 MB).
- **Why:** Store purge mid-sweep fights MuPDF caching; not a viable production strategy.
- **Changed:** **No production store purge**; carry forward as hazard.

- **Decision/Issue:** Do **not** claim lazy loading “fixed 294→43 MB”.
- **Why:** That delta was a **measurement-window artifact** (reader window vs full pass), not an optimization win.
- **Changed:** Knowledge and memory updated; reviewer nit on slice README wording carried forward (do not edit README in this close-out).

- **Decision/Issue:** `progress.md` SHA pattern on 01/05 — gate-fix commit `1fb4242` then separate docs commit `883a98c` before merge.
- **Why:** Extra commit after gate fix is the correct pattern vs amending; slice HEAD for review/CI was `883a98c`.
- **Changed:** Memory lesson appended; recurring progress SHA lag entry refined.

## 2026-08-20 (01/04 close-out)

- **Decision/Issue:** Slice 01/04 backend-parity-hardening merged to `dev` via PR #8 (slice HEAD `153cafc`, merge `3d519af`). Independent reviewer APPROVE (round 2b); round 1 REQUEST CHANGES (`_open_fn` typo; dishonest Unsupported self-match). CI run `32387915511` on `153cafc`: all 7 jobs success.
- **Why:** Phase 01 fourth slice — shared parity suite, hardening matrix, first-page text sampling for `NoTextLayer`, cross-backend fixture expectations.
- **Changed:** `tests/parity/`, `textlayer.rs` sampling, fixture helpers; Option B Unsupported (factory gating only — PDFium never maps open to Unsupported); textlayer zero-page → `Malformed`; compile skip `let _ = open_fn`. Handoff advances to 01/05.

- **Decision/Issue:** Round 1 caught dishonest Unsupported self-match — test asserted Unsupported via factory gate, not engine behavior.
- **Why:** Parity slice must pin engine outcomes, not factory wiring tricks.
- **Changed:** Option B: Unsupported only via factory gating; PDFium open path never returns Unsupported; reviewer round 2b APPROVE.

- **Decision/Issue:** Hardening table row “unsupported PDF feature → Unsupported” deferred.
- **Why:** No honest cross-backend fixture/policy without factory-only gating; 01/04 scoped to provable parity cases.
- **Changed:** Factory gating only; open question carried to later slice.

- **Decision/Issue:** `progress.md` still cites commit SHA `1b4b613` after 01/04 merge.
- **Why:** Third recurrence of progress SHA lag — row written before final slice commit or not updated at merge close-out.
- **Changed:** 01/05 builder must set 01/04 merged + merge SHA; memory updated with recurring-pattern lesson.

## 2026-08-20 (01/03 close-out)

- **Decision/Issue:** Slice 01/03 pdfium-backend merged to `dev` via PR #7 (feat `b3d1326`, merge `b9902d0`). Independent reviewer APPROVE (nits only). PR CI run `32383125922`: all 7 jobs success.
- **Why:** Phase 01 third slice — permissive PDFium backend behind feature, Arc engine, fixture parity with MuPDF slice, CI libpdfium delivery.
- **Changed:** `PdfiumBackend`, `pdfium.rs` tests, CI libpdfium fetch/cache/checksum + `PDFIUM_LIB`, `pdfium-render` with `sync` feature; handoff advances to 01/04.

- **Decision/Issue:** Parallel pdfium tests SIGABRT under `cargo test` — `thread_safe` feature alone insufficient.
- **Why:** pdfium-render/PDFium C API not safe for concurrent ops from multiple test threads sharing one engine.
- **Changed:** Global `PDFIUM_OPS` mutex serializes pdfium operations; tests pass in parallel.

- **Decision/Issue:** Zero-page PDF: PDFium returns `FPDF_ERR_FORMAT` while MuPDF opens with `page_count == 0`.
- **Why:** Engines disagree on malformed vs empty catalog for `zero-pages.pdf` fixture.
- **Changed:** `is_zero_page_catalog` path sniff maps PDFium FORMAT → `Malformed` for parity; reviewer nit — fixture-specific, harden or document in 01/04.

- **Decision/Issue:** `Arc<Pdfium>` / `Document` failed Send+Sync without `sync` crate feature.
- **Why:** `pdfium-render` gates Send+Sync impls behind optional `sync` feature.
- **Changed:** `Cargo.toml` enables `sync` alongside `pdfium_latest` and `thread_safe`.

- **Decision/Issue:** `progress.md` recorded commit SHA `9892010` before feat commit amended to `b3d1326`.
- **Why:** Progress row written before final amend — SHA stale in progress until 01/04 builder updates.
- **Changed:** Lesson: never write progress.md commit SHA until slice commit is final.

- **Decision/Issue:** libpdfium pin is bblanchon `chromium/7543` (`pdfium_7543`), not slice README marketing version `153.0.8009.0`.
- **Why:** `pdfium-render 0.8.37` `pdfium_latest` maps to chromium/7543 ABI; marketing Chromium version ≠ pdfium-binaries tag.
- **Changed:** CI downloads `chromium%2F7543` tarball, sha256 `2383a414050dd21ae5300b119ad8a72360ef92cff820b4c685c047dc272c2794`.

## 2026-08-20

- **Decision/Issue:** Slice 01/02 mupdf-backend shipped to `dev` via PR #5 (commits `ab08f07` feat, `e801982` fix, `af7441f` progress, `ad83c7b` merge, `d8dae1f` history).
- **Why:** Phase 01 second slice — real MuPDF backend behind feature, factory off Stub, error mapping and fixture coverage per plan.
- **Changed:** `MupdfBackend`, `zero-pages.pdf` fixture, deny.toml expand, tiny/blank/zero fixtures, progress.md row.

- **Decision/Issue:** Independent reviewer round 1 REQUEST CHANGES on 01/02 — false `_ctx` ownership narrative; zero-page → `Malformed` guard unproven.
- **Why:** Reviewer caught doc/code mismatch and assertion without fixture.
- **Changed:** Ownership wording corrected to TLS `fz_context` honesty; `zero-pages.pdf` proves silent 0-page open maps to `Malformed`; round 2 APPROVE.

- **Decision/Issue:** CI broke on `dev` after 01/02 merge — `yeslogic-fontconfig-sys` build failed on ubuntu-latest (missing `fontconfig.pc`).
- **Why:** MuPDF dependency needs native fontconfig headers/pkg-config on Linux; local dev machines often already have them; cold CI runner did not.
- **Changed:** PR #6 (`ba32ef6`): `sudo apt-get install -y libfontconfig1-dev pkg-config` on `rust-checks` and `bench` jobs in `.github/workflows/ci.yml`.

- **Decision/Issue:** Cursor harness parity completed for multi-harness agent routing.
- **Why:** `.agents/` is canonical; Cursor and OpenCode need identical symlink adapters, not parallel agent defs.
- **Changed:** `sync.sh` writes 11 symlinks each to `.cursor/agents/` and `.opencode/agent/`; `remove.sh` cleans both; `orchestration.mdc` `alwaysApply: true` + Mia default; README documents restart/new-chat tip.

- **Decision/Issue:** PR #6 merged to `dev` (merge SHA `98a4744`, mergedAt 2026-08-20T14:21:38Z). Prior knowledge entries described the fontconfig CI fix as merged while GitHub still showed the PR open.
- **Why:** Close-out was written before `gh` merge completed — knowledge lagged GitHub reality by one step.
- **Changed:** Post-merge CI run `32379667373` on `98a4744` — all 7 jobs success; `dev` trustworthy green. Knowledge reconciled; handoff now reflects merged state and 01/03 in progress.

## 2026-08-20 (standing rule + 01/03 post-merge CI)

- **Decision/Issue:** User standing rule — Mia may approve worker sandbox/network/git_write and slice PR merge to `dev` without user round-trip; destructive git and 01/11 tag/dogfood/main-merge still need user.
- **Why:** Phase-01 cadence stalls when subagents bounce permission prompts to the human; orchestrator already owns delegation and merge sequencing.
- **Changed:** memory, handoff, status, log updated; orchestration skill Guardrails one-liner. Post-merge CI run `32384136200` on `b9902d0` verified green (all 7 jobs) — handoff hazard cleared.

## 2026-08-25 — v0.1 UX-perfection iteration (slice/02-01-gui-reader)

- **What:** Lucide SVG chrome icons (egui_extras `svg`, ISC, assets
  committed), spacing pass (bar heights, flat chrome, rail rhythm, GAP 16 /
  MARGIN 20), and five layout bug fixes: sidebar section panel stacking,
  nav-cluster LTR order, rail gear pinning (three attempts, settled by
  eprintln instrumentation: horizontal uis clamp children to ~18 px), gear
  spacer math, TOC right-aligned page numbers.
- **Why:** font-glyph chrome rendered tofu boxes and mirrored `‹›` via font
  fallbacks; hand-drawn painter icons failed the quality bar; spacing was
  default-egui cramped versus the mockup.
- **Root causes worth keeping:** (1) `ui.horizontal` clamps child height to
  `interact_size.y` — give full-height rows an explicit `set_height`;
  (2) `Layout::bottom_up` allocates from the cursor, not the rect bottom;
  (3) `pkill -f` with a path regex self-matches the driving shell — the
  long-running "stuck builder" plague; `pkill -x candi` instead;
  (4) a missing fixture PDF shows the error card — earlier "broken UI"
  captures were partly error states and blank synthetic pages.
- **Process:** worked as direct dictation — exact-string python patches
  with count asserts, build → shot.sh → visually read every PNG before the
  next change. That loop is what found the real bugs; keep it.
- **Release flow (user directive):** slices merge to `dev`; every 0.x tag
  lands on `main`.

## 2026-08-26 — six-state reference rebuild (slice/02-01-gui-reader)

- **What:** chrome rebuilt to the user's six-state image: rail restored as
  its own SidePanel, hamburger/wordmark back, tagline + duplicate zoom
  stepper removed, Inter embedded, dark-first, empty state styled, Lucide
  icons, exact_width panels, central-rect backdrop paint, PID-matched
  shot.sh.
- **Why:** the earlier rebuild followed the stale light mockup (nav rows, no
  rail, no wordmark) — wrong target; the six-state image is authoritative.
- **Root causes worth keeping:** (1) stale fixture + captured wallpaper/
  overlay windows masqueraded as UI bugs — always verify WHAT was captured
  (pid-matching now mandatory); (2) horizontal-layout height clamping keeps
  biting — full-height content belongs in panels; (3) add_sized centers;
  (4) `pkill -f` self-matches — the old "stuck builder" plague.
- **Process:** iterate build → PID-matched capture → actually read the PNG →
  fix. Ten-plus rounds; each visual defect (mirrored chevrons, tofu,
  maroon band, side-by-side TOC, gear placement, centered titles, reversed
  steppers) was caught by reading captures, not by reading code.
- **Deferred by user:** search overlay, Appearance panel, window
  decorations, dual/mobile modes, disk cleanup (opencode.db 26G).

## 2026-08-26 (later) — seam chase + revert

- **What:** five more capture-verified rounds: killed the transparent strip
  class (central frame fill, zero-margin panel frames), restored hamburger +
  wordmark + rail per the six-state reference, deduped zoom controls,
  removed the tagline, tightened canvas margins, PID-hardened shot.sh.
- **Reverted:** `2231aa8` embedded the section panel inside the central
  surface to kill the panel seam by construction — it reintroduced the
  horizontal height clamp (blank panel, clipped canvas). Reverted in
  `8ee2110`; the SidePanel structure is correct, the remaining seam is a
  ~1px divider + shadow, documented as a judgment call in the handoff.
- **Lesson:** when a cosmetic fix requires restructuring working layout,
  revert fast and document — three rounds of seam chasing bought two
  regressions; the SidePanel version was one good review away from done.

## 2026-08-26 (night) — feature-complete push

- **Shipped in sequence:** install (PATH symlink + desktop entry; stale
  cargo binary shadowed it), named bookmarks (schema v3, hand-rolled TOML
  parse adapted for optional titles), Appearance panel (rail gear; accent
  swatches apply via name-cache sync hack; text size = pixels_per_point
  with reopen reset piggybacked on the `primed` hook), search overlay
  (overlay owns the query field; panel is a pure results list), backend
  rect search + in-page highlighting (MuPDF TextPage::search quads
  top-left; PDFium find loop flipped y-up→y-down; orientation proven by a
  probe against word coords, then deleted), page flows (rows-of-k layout,
  page_at returns row-first, fit-width over widest ROW), in-app window
  decorations (Minimized(bool) not Minimize; drag on the brand zone).
- **Recurring root causes:** egui panel-seam class ended in a reverted
  experiment (2231aa8) — keep SidePanels; patch scripts must be authored
  against post-fmt state; capture evidence must be PID-matched and
  actually READ before concluding.

## 2026-08-27 (v0.1.0 candidate close-out)

- **Decision/Issue:** v0.1.0 candidate assembled on `slice/02-01-gui-reader`
  — 8 commits `a213302..b06ff39` atop origin `3866fa6`, all local/unpushed.
  Search crash fix, chrome/sidebar polish (15 items), anchor-preserving
  zoom/flow/resize/pinch, YAML theme editor + Dark default + prefs/recents/
  accent retint, keybinds.json + custom themes, docs/architecture correction
  + new `docs/roadmap.md`, AUR packaging, review round-2 doc fixes.
  Independent review round-2 verdict FIX-FIRST on 5 docs/regression items →
  all fixed in `b06ff39`; targeted re-review verdict pending at close-out.
- **Why:** finish the GUI reader slice to shippable quality; every fix below
  came from dogfood rounds or review, not speculation.
- **Changed (root causes worth keeping):**
  - *Char-boundary panic:* scan_one_page byte scan sliced mid-codepoint on
    accented page text → UI thread panic. Fix = `as_bytes()` compare +
    `is_char_boundary` guard + café/résumé regression test. ASCII-only
    fixtures structurally hide byte-index bugs.
  - *egui 0.30 batch:* `Layout::main_align` aligns inside a rect, not widget
    streams (center the pill via screen-anchored `Area`);
    `allocate_ui_with_layout` returns child min_rect (right edges need exact
    rects + `ui.put`); Inter lacks ←→ glyphs (use words); slider rail+handle
    both paint `widgets.inactive.bg_fill` → detached-dot illusion, fixed by
    `trailing_fill(true)`; `Modifiers` not Hash (decompose to bools);
    `widgets.open.weak_bg_fill` must be themed (dark-on-dark headers in Light).
  - *Keybind shift parity:* OS-level combos typed with shift produce different
    chars (Shift+= → '+'). Kept exact-modifier matching strict; enumerated
    spelling variants (`"+","=","Shift+=","Ctrl+=","Ctrl++","Ctrl+Shift+="`)
    in DEFAULTS instead.
  - *Docs-from-memory fabrication:* the sidecar TOML example had invented
    fields until checked against `serialize_session` — always quote
    serializers, never reconstruct examples from recall.
  - *Packaging:* makepkg injects `-flto` → lld cannot resolve bundled-C
    bitcode at rustc link; `options=('!lto')` mandatory; pin RUSTFLAGS
    (spurious `-C target-cpu=native` observed once — portability hazard);
    check() needs backend exclusions when a dlopen-ed lib isn't packaged.
  - *Shell hygiene:* backgrounded GUI launch inheriting stdout FD hangs the
    driving tool — `nohup … < /dev/null & disown`.
  - Minor: `warn()` helpers taking String force `.to_owned()` call-site
    warts — future helpers take `impl Display` (2-line cleanup candidate).
- **Benchmarks recorded** (7 unique real books `/mnt/personal/Books`,
  0.8–50 MB, 148–1556 pp): core `reader_peak` avg mupdf 17.6 MB / pdfium
  18.4 MB, worst 58 MB (silberschatz pdfium) → 200 MB gate passes ~3.4×
  headroom; full_pass hazards NOT gated (sdi2 pdfium 1996 MB image sweep;
  silberschatz mupdf 295 MB TLS store); GUI process RSS ~190 MB mupdf /
  ~213 MB pdfium vs egui floor ~170 MB; open+first render avg 61 ms mupdf /
  23 ms pdfium.
- **User decisions captured:** versioning — majors are `v0.x` tags landing
  on main; post-release fixes `v0.x.y`; features → dev; new feature branches
  only AFTER each version cut. Roadmap v0.1 (base+polish, HERE) → v0.2
  epub/password PDFs/optimizations → v0.3 Windows/other Linux/deployments →
  v0.4 GitHub docs/cleaning → v0.5 AUR/Debian+CI (AUR pulled EARLY per user;
  package ready in `packaging/`) → v0.6 Android beta. Infra flags: dev/main
  UNPROTECTED (decision pending); `origin/slice/docs-catchup` prune candidate
  (needs user); tag MUST literally be `v0.1.0` (PKGBUILD URL); Cargo.lock
  conflicts expected with any revived cargo dependabot PR (none open;
  dependabot PR #1 closed w/ comment, manual consolidated checkout bump
  planned post-merge); `/tmp` tmpfs 7.7G overflows Rust target dirs → use
  `/mnt/personal/tmp` (SIGBUS ×2 sessions).

## 2026-08-28 (GUI-only pivot)

- **Decision/Issue:** Public surfaces pivoted GUI-only (`754d73d`) —
  README, `docs/roadmap.md`, PKGBUILD, `.SRCINFO`, `AUR.md` de-advertise
  `candi-tui`.
- **Why:** User intent: v0.1.0 ships the `candi` GUI reader; the TUI is
  internal and must not be publicly promised.
- **Changed:** Docs/packaging rewritten. Incident: the user hand-edited
  `packaging/.SRCINFO` pkgdesc and an agent git-restored the file as
  anomalous, losing the edit. Rule: NEVER silent-restore an unexpected
  working-tree change — ask the user or match stated intent first
  (memory 2026-08-28).

## 2026-08-28 (icon branding)

- **Decision/Issue:** Icon branding adopted (`ce4486b`): rounded
  21%-radius multi-res hicolor set (16–512) for packaging/desktop +
  embedded 256px window icon.
- **Why:** Product identity across desktop entry, AUR package, and window
  chrome.
- **Changed:** hicolor size set + desktop icon, embedded PNG via
  `image 0.25` with png-only feature.

## 2026-08-28 (dogfood batch — 19 issues fixed in c507c15)

- **Decision/Issue:** Dogfood surfaced 19 issues; all root-caused and
  fixed in `c507c15`.
- **Why (root causes):** keybinds editor focus-steal (egui TextEdit never
  blurs on outside click) + stale v1 seeds → FocusGuard +
  `DEFAULTS_SCHEMA_VERSION` 2 migration healing; render state machine had
  3 leak paths → FailState ledger with 250ms→1s→4s backoff +
  click-to-retry + stale-scale pruning; TOC jumps wrong (MuPDF XYZ top /
  PDFium `FPDFDest_GetLocationInPage` + y-flip) + accent same-page
  boundary → `dest_top` end-to-end + `toc_follow` clicked-row preference;
  ANR-class stalls = sync search + blocking rfd dialog → async
  `SearchSession::step` worker (mpsc + AtomicBool) + non-blocking
  single-flight dialog; startup detoured through pick_pdf → straight to
  welcome.
- **Changed:** `c507c15`; page toast 700ms/200ms. Pinch-zoom deferred:
  winit has NO Linux pointer-gesture support (Wayland
  `zwp_pointer_gestures` side-channel possible; Ctrl+scroll works and is
  noted in the shortcuts window) — v0.1.1 backlog.

## 2026-08-28 (review battery + hardening 8960d1a)

- **Decision/Issue:** Pre-tag review battery: independent round-2
  worktree review PASS; security FIX-THEN-PASS; silent-failure-hunter
  PASS.
- **Why:** Security found 1 MED crash-DoS (unbounded outline recursion →
  depth cap 64, MuPDF parity) plus LOWs (fixed or deferred);
  silent-failure found H1 data-loss (corrupt-sidecar save path) + 4 MEDs.
- **Changed:** Hardening commit `8960d1a`: outline depth cap 64,
  corrupt-sidecar `session_corrupt` flag + banner, failed-open-over
  preserves sidebar, search `catch_unwind`, theme-delete keeps entry on
  failure, PDFium missing-render-symbol → error, `dest_top` is_finite
  guards, FIFO preflight `is_file`, PoisonError recovery, theme YAML
  256 KiB cap, dead-code nit. All fixes verified by gates.

## 2026-08-28 (main/dev divergence at release)

- **Decision/Issue:** `main` had diverged from `dev` when the release
  merge ran; reconciled at `dev`→`main` merge `8995b64`.
- **Why:** The release-flow precondition (main must be ancestor of dev, or
  divergence reconciled) was checked only at merge-2, not merge-1 — cost
  one aborted merge cycle (clean abort, but late).
- **Changed:** Doc conflicts resolved: `log.md`/`progress.md` UNIONED both
  eras (phase-00 records kept); `handoff.md`/`status.md` took `dev`.
  Lesson: check preconditions BEFORE merge-1 (memory 2026-08-28).

## 2026-08-28 (v0.1.0 cut + private-repo blocker)

- **Decision/Issue:** v0.1.0 cut: PR #21 merged into `dev` (`f864a15`),
  `dev`→`main` `8995b64`, annotated tag `v0.1.0` pushed (→ `8995b64`).
- **Why:** First release per `docs/roadmap.md`; tag must be the literal
  `v0.1.0` (PKGBUILD source URL depends on it).
- **Changed:** Blocker discovered — repo is PRIVATE, so the tag-tarball
  source URL 404s anonymously; AUR push blocked until the user flips
  visibility public. AUR one-time prep + day-of steps handed to the user.
  Lesson: check `gh repo view` visibility before promising AUR readiness
  (memory 2026-08-28).

## 2026-08-28 (v0.1.1: release pipeline + install paths + AUR deferred)

- **Decision/Issue:** v0.1.1 shipped same day as v0.1.0: release binary
  pipeline + install paths + AUR deferred to post-multi-OS (user decision,
  no AUR account yet). `dev` commits `e198101` (release.yml: tag+dispatch
  triggered, ubuntu-22.04, clang+pkg-config+libfontconfig1-dev, RUSTFLAGS
  debuginfo=0, strip, dist tarball + sha256 sidecar, `gh create/upload
  --clobber`, no third-party release action), `16a486a` (workspace 0.1.1
  via version.workspace single bump; candi-gui description+repository
  metadata — cargo package now warning-free), `b2cb451` (README install
  paths: release download+sha256, `cargo install --git … --tag v0.1.1
  --locked candi-gui --bin candi` — explicit package selector REQUIRED on
  a two-bin workspace; from-source path; AUR deferred line; PKGBUILD
  pkgver 0.1.1 + .SRCINFO regen; POSIX packaging/install.sh), `558a28b`
  (architecture.md root-relative → ../crate links), `00838ff` (merge
  origin/main into dev — the repo historically NEVER merges main back
  into dev, so the standard main-is-ancestor-of-dev release precondition
  was structurally unfulfillable; merge-back performed at cut time;
  merge-tree predicted == actual tree). Release: `dev`→`main` `67381d2`,
  annotated tag `v0.1.1` (→ `67381d2`), CI green on main, release.yml
  green (~6 min), https://github.com/mehboobulqadri/candi/releases/tag/v0.1.1
  with candi-0.1.1-x86_64-linux-gnu.tar.gz (12,419,211 B) + .sha256;
  artifact verified (sha256sum -c OK; tarball contains binary +
  install.sh + INSTALL.md + desktop + 8 icons; ldd max GLIBC_2.35 →
  Arch/Debian 12+/Fedora 36+ fine); headless CANDI_NO_GUI checks pass
  (missing-path rc=1; fixture open page=1 rc=0 — fixture COPIED to tmp
  first because headless writes .candi.toml sidecars next to the PDF).
  Repo still PRIVATE → release URL not anonymously downloadable until
  the user flips visibility.
- **Why (root causes hit):** (1) docs-links CI failed on root-relative
  links — the checker resolves relative to the .md file's directory, so
  repo-root-relative links from docs/ silently break CI; (2)
  `cargo install --git` on a multi-bin workspace needs an explicit
  package selector (`candi-gui --bin candi`) — `--bin` alone errors
  "multiple packages with binaries found"; (3) main was never merged
  back into dev after cuts, so the main-is-ancestor-of-dev precondition
  could never hold — merge-back performed at cut time (`00838ff`), now
  convention after EVERY cut.
- **Changed:** Release flow gains a merge-main→dev step; `packaging/`
  stays validated and ready (AUR day-of steps in packaging/AUR.md); AUR
  moves to roadmap v0.3 (v0.5 Debian-only+CI); new backlog items a–f in
  handoff; verification found `main` currently one merge commit AHEAD of
  `dev` (dev is ancestor of main) — next merge-back pending.

## 2026-08-28 (local machine refresh)

- **Decision/Issue:** `~/.local/bin/candi` symlink had gone MISSING
  entirely — recreated → `/mnt/personal/Projects/candi/target/release/candi`
  (in-repo release build, no CARGO_TARGET_DIR, so the symlink auto-updates
  on rebuilds); all 8 rounded RGBA icons + candi.desktop installed to
  `~/.local` (corner alpha 0 verified); no live instance during the swap.
- **Why:** The missing symlink broke PATH/desktop launch; the v0.1.1
  rounded icon set needed installing.
- **Changed:** Binary exposes NO `--version` flag (clap
  disable_version_flag) — version visible in GUI about as
  "v0.1.1 — AGPL-3.0"; enabling it is backlog item (f).

## 2026-08-28 (v0.1.2-RC: fix wave, README face v2, repo hardening)

- **Decision/Issue:** v0.1.2-RC assembled on `dev` == `origin/main` ==
  `fdb9656`; tag NOT cut; awaiting USER DOGFOOD. Fix wave: `a285bcd`
  (Linux Ctrl+ keybinds + zoom-seed heal), `7b247d1` (async open,
  unfocused repaint gate, toast, TOC cursor, Esc scoping, honest pinch
  copy), `aeca428` (publishing.md removed), `666d08f` (first-run
  onboarding, `[misc] onboarding_done`). Repo hardening: PR #29
  (`209071e`→`fdb9656`) README face v2 with full-window theme screenshots;
  PR #22 (`daff662`) product README + canonical AGPL restored; lean CI
  (ci.yml 4 jobs/push, deny.yml weekly+dispatch, bench.yml dispatch-only,
  release tar.gz+zip+sha256s); `skills-lock.json` moved into `.agents/`;
  dependabot DISABLED (`.github/dependabot.yml` removed in `0314ede`,
  PRs #23–28 closed, 6 dependabot remote branches linger); rulesets
  replace classic branch protection (main-protection 21686764: PR-only,
  0 approvals, 4 required checks, no bypass incl. admin, force/delete
  blocked, `require_extra_approval_for_unattributed_changes=false`
  Mia-approved for solo flow; dev-protection 21686769: force/delete
  blocked, admin bypass always; tags/releases unaffected).
- **Why (root causes):** (1) ALL Linux Ctrl+ bindings were dead —
  `Binding::matches` compared `pressed.command` and egui-winit aliases
  `command = ctrl` on Linux; tests now use platform-real modifier shape;
  stale 2-spec zoom seed `["+","="]` healed by treating missing
  `schema_version` as v1 in `migrate_document`. (2) ANR on 1556pp books —
  open_session + page-size sweep + outline ran on the UI thread; moved to
  a `candi-open` worker with single-flight + cancel, `prime` stays on UI;
  sidebar preserved on failed open. (3) Unfocused egui windows repainted
  on timers — `should_schedule_repaint` gates timed repaints while
  unfocused, async results surface on refocus. (4) Toast resurrected on
  the same page and anchored under the bar — `last_toasted_page` memory +
  anchor sign fixed to `-(BOTTOM_BAR_HEIGHT + 12)`. (5) Dependabot PR
  churn (6 open PRs) wrong cost/benefit for a solo maintainer — disabled;
  caching bumps already merged manually (#20).
- **Changed:** v0.1.2 next steps fixed: dogfood checklist → PR dev→main
  → merge main back into dev → tag LITERAL `v0.1.2` on the main merge
  commit → download tar.gz AND zip, extract, run install.sh per README
  (user-mandated final validation) + verify sha256s. Local user binary
  rebuilt at `4b6adc8+` via symlink. Handoff + status rewritten.

## 2026-08-28 (incident: pushed to dev before gates → CI red)

- **Decision/Issue:** One cancelled run PUSHED TO DEV BEFORE GATES — CI
  went red on `0314ede`: 3 prefs tests shared the
  `candi-prefs-test-{pid}` temp dir and raced.
- **Why:** Cancellation pressure compressed the loop; the gate step was
  skipped instead of the push being held.
- **Changed:** Fix `4b6adc8` (distinct temp dirs per test + 10× stress).
  RULE: gates BEFORE push, no exceptions, even under cancellation
  pressure — a cancelled job's work stays unpushed until green locally.

## 2026-08-28 (incident: two workers shared a filesystem path)

- **Decision/Issue:** Two parallel workers shared a filesystem scope —
  the cleanup worker deleted `/tmp/candi-shots` while the README worker
  was still inspecting it (harmless this time; it held stale captures).
- **Why:** Task boundaries were by deliverable, not by path; nothing
  arbitrated shared directories.
- **Changed:** RULE: never give two workers overlapping paths — each
  worker gets a private tmp scope, and cleanup of shared dirs waits until
  no other worker references them.

## 2026-08-28 (machine: RAM/swap crash wave + disk cleanup pending)

- **Decision/Issue:** RAM/swap crash wave on this box. Root causes:
  (1) big parallel builder work; (2) tmpfs `/tmp` accumulation (1.9G
  candi caches); (3) `~/.local/share/opencode/opencode.db` ~27G + WAL
  ~17G — LIVE, cleanup DEFERRED to the user post-session (stop opencode →
  `PRAGMA wal_checkpoint(TRUNCATE); VACUUM;` → if space-blocked, rm
  db+wal+shm after closing → restart; ~40G back).
- **Why:** Parallel cargo builds + tmpfs bloat + a 44G live WAL left no
  headroom; crashes were resource exhaustion, not code.
- **Changed:** 34.6G freed from `/mnt/personal/tmp` + 1.9G tmpfs; big
  target dirs stay on `/mnt/personal/tmp`, never tmpfs; parallel build
  scope reduced; status.md carries the pending-cleanup constraint until
  the user runs it.

## 2026-08-28 (v0.1.2 cut + final wave)

- **Decision/Issue:** Final wave on `dev` then v0.1.2 cut. Wave: `81074a6`
  (page-turn +/-/= rebinds + defaults schema v3 exact-match seed heals +
  Ctrl-only zoom), `de8bfc5` (six new themes: Cyberpunk, Catppuccin, Nord,
  Dracula, Gruvbox Dark, Solarized Light), `f3ddff5` (chrome-focus sweep +
  toast single-line CENTER_BOTTOM over captured `zoom_slider_rect` +
  96MB-budgeted per-palette recolor LRU), `ef6ac20` (0.1.2 bump +
  SECURITY.md supported-versions fix), `cde66b5` (review-nit sweep:
  `RecolorKey.theme`→`colors` rename + shortcuts a11y note). Reviewer
  verdict: SHIP — zero blockers/majors; 5 nits swept or deliberately kept
  (Dracula `#343746` / Solarized Light `#E4DCC6` panel_bg kept as
  deliberate soft-chrome). Release: PR #30 dev→main merged, merge sha
  `37362d9` = tag `v0.1.2`; release.yml assets tar.gz + zip + sha256s all
  verified via `sha256sum -c`; install.sh run per README verbatim
  (`~/.local/bin/candi` is now the RELEASE binary, not the dev symlink);
  smoke `CANDI_NO_GUI=1 candi --help` OK; `dev` == `main` == `37362d9`.
  User feedback: keybinds verified working (+/- page turns, Ctrl+-
  app zoom; Shift+- also pages — accepted).
- **Why (root causes worth keeping):** (1) "q doesn't quit" — egui's
  Tab/arrow focus navigation parked focus on chrome widgets, so
  `wants_keyboard_input()` ate ALL plain keys until Esc; fix = per-frame
  focus sweep that surrenders transient focus on chrome, keeping focus
  only on registered real text-edit fields. (2) Theme-toggle ~1s stall —
  `set_theme` cleared all texture slots, so the next frame re-promoted
  ~10 candidates with full 20MB/page clone + LUT recolor + upload
  (≈200MB in one frame); fix = 96MB-budgeted per-palette recolor LRU so
  recolored textures survive theme switches. (3) Incident-shaped note:
  first clippy run failed with `Unrecognized option: 'j'` — `-j 6` was
  placed after `--` and passed to rustc; cargo job flags go BEFORE `--`.
- **Changed:** v0.1.2 live; `~/.local/bin/candi` swapped to the installed
  release binary; v0.1.3 scope opened (pinch-to-zoom side-channel in
  flight, scroll-sensitivity slider, smooth page-turns) — handoff
  rewritten, status updated.

## 2026-08-28 (pinch-to-zoom research conclusion)

- **Decision/Issue:** Trackpad pinch-to-zoom cannot work through winit on
  Linux: winit 0.30.13 `PinchGesture` is macOS/iOS-only and its Wayland
  backend has no `zwp_pointer_gestures_v1` binding (Hyprland 0.56.2 DOES
  serve the protocol; egui-winit's PinchGesture→`Event::Zoom` is dead
  code on Linux). Industry survey: Chromium ozone
  `wayland_zwp_pointer_gestures` is the reference pattern (absolute scale
  → per-update `scale_now`/`scale_prev` multipliers, cancelled-end
  no-op); GTK/GDK binds internally (Firefox works via GDK since FF88);
  Qt never shipped it; no Rust project has it.
- **Why:** The protocol exists and the compositor serves it — only the
  winit layer is missing; upstream `winit-extras` (#2160) is an option
  but not on our timeline.
- **Changed:** Fix = in-crate side-channel: `raw-window-handle` +
  `wayland-client` `from_external_display` (smithay-clipboard pattern),
  thread-owned queue → mpsc → `set_zoom_percent`; est 2–4 days. Now
  being implemented for v0.1.3 (debug hook `CANDI_GESTURE_DEBUG=1`;
  shortcuts copy flips from "not supported" only once proven).

## 2026-08-28 (user reports: q quit hangs UI + more v0.1.3 feedback)

- **Decision/Issue:** Three NEW user reports after the v0.1.2
  close-out. (1) CRITICAL regression in RELEASED v0.1.2: pressing plain
  `q` (quit) FREEZES the UI; after a while the desktop shows the Wayland
  unresponsive-app dialog ("terminate or wait"); the user must kill
  candi manually. NOT yet diagnosed — investigation tasks were
  cancelled 4× before running. Ranked hypotheses to carry: (a)
  synchronous save/teardown on the UI thread between quit request and
  eframe exit (session sidecar write, prefs save, texture/cache
  teardown); (b) join/wait on the `candi-open` worker with no timeout;
  (c) transient_focus sweep interaction (least likely). Replication
  plan for the next agent: sandboxed `XDG_CONFIG_HOME` under
  /mnt/personal/tmp, run `target/release/candi` under
  `strace -f -tt -T` from launch (parent ⇒ ptrace OK), send `q` via
  `hyprctl dispatch sendshortcut` (check Hyprland 0.56 syntax) or
  `wtype`/`ydotool`, read the syscall tail (futex = lock/join; write
  storm = save), `pkill -x candi` never `-f`. (2) Toast: user wants it
  horizontally centered on the WHOLE window (currently centered over
  the zoom slider since f3ddff5), SAME vertical line as now (the
  bottom-bar page-info line). Small change in app.rs
  `show_page_toast`/`toast_offset` + `toast_centers_over_the_zoom_slider`
  test rename/update. (3) Scroll: mouse WHEEL scrolling is still too
  slow (in addition to trackpad) — v0.1.3 scope grows: raise
  wheel+trackpad defaults and add a settings slider; check egui 0.30
  `InputOptions` scroll knobs for a trivial mapping.
- **Why:** Released v0.1.2 has a shipping-critical exit-path hang that
  is undiagnosed, and two smaller UX corrections; the log must carry
  the hypotheses + replication plan so no investigation restarts from
  zero after the 4 cancellations.
- **Changed:** `handoff.md` gains "Open regressions + newest feedback
  (TOP PRIORITY)" before the v0.1.3 scope; v0.1.3 scope item 2 renamed
  "Scroll sensitivity (wheel + trackpad) + settings slider" (wheel
  sensitivity folded in); memory gains the small-tasks orchestration
  lesson.
