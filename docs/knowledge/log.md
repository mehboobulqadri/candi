# Log

Append-only project history: decisions made, issues faced, root causes.
Never edit entries in place — append below the marker with a date header.

Format per entry:

## YYYY-MM-DD

- **Decision/Issue:** what happened.
- **Why:** the reasoning or root cause.
- **Changed:** what changed because of it.

<!-- entries below -->

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
