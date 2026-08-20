# Candi — Implementation Progress

Current position: 01-v0.1/09-candi-tui committed; next:
`01-v0.1/10-candi-cli`.

## Status

| Phase | Slice | Status | Notes |
|---|---|---|---|
| base | docs (repo docs + spike results + this plan) | merged | lands as the repo's initial commit |
| 00-foundations | 01-workspace-bootstrap | merged | workspace skeleton + CI workflow fix (hashFiles job-level guard bug) |
| 00-foundations | 02-ci-drill-gates | merged | feature-mode matrix (grow-ready, single mode until features land), cargo-deny + deny.toml, target cache keyed on Cargo.lock |
| 00-foundations | 03-benchmark-harness | merged | bench harness (std-only bench binary + run.sh); committed fixture is dummy-encrypted only (real books local-only per repo policy); runtime-generated fixtures, corpus manifest, bench CI job |
| 01-v0.1 | 01-candi-pdf-trait | merged | pinned trait block verbatim (8-kind Error, Document/Backend, PagePositions); factory + features declared (empty deps); StubBackend + 23 tests; both feature modes green; permissive matrix entry activated |
| 01-v0.1 | 02-mupdf-backend | merged | `mupdf-backend` feature (mupdf 0.8.0 base14-fonts); MupdfBackend + fz_context ownership; error mapping by `fz_error_code`; zero-page open → `Malformed`; fixture tests (attention paper, truncated, encrypted); fix commit proves blank-first-page vs zero-pages |
| 01-v0.1 | 03-pdfium-backend | merged | b3d1326 — pdfium-render 0.8.37 (pdfium_7543 / chromium/7543); Arc engine + FPDF_DOCUMENT drop; permissive build; CI libpdfium pin + PDFIUM_LIB; merge b9902d0; independent reviewer APPROVE |
| 01-v0.1 | 04-backend-parity-hardening | merged | 153cafc — shared parity suite + hardening matrix; open-time text-layer sampling (first 3 pages); merge 3d519af; independent reviewer APPROVE (round 2b after REQUEST CHANGES) |
| 01-v0.1 | 05-benchmarks-both-backends | merged | fc5a2af — budget gate uses reader_peak (<200 MB); full_pass_peak ~295 MB documented FAIL vs MuPDF store ceiling, not gated; merge c3634e0; PR #10 |
| 01-v0.1 | 06-candi-core-navigation | merged | merge 994a154 (PR #11); feat 8b955fa — ViewState page+scroll, clamping nav, caller max_scroll; 15 tests |
| 01-v0.1 | 07-reading-position-sidecar | merged | merge decce47 (PR #12); feat bd440c3 — schema v1 sidecar `{pdf}.candi.toml`, Load enum (missing/corrupt/loaded), atomic temp+rename save, 11 tests |
| 01-v0.1 | 08-search-abstraction | merged | a77a874 — lazy SearchSession over Document, case-insensitive per-page scan, cursor wrap, 16 tests |
| 01-v0.1 | 09-candi-tui | committed | 238d410 — ratatui 0.30.2 + crossterm 0.29.0 TUI reader, TestBackend 14 tests, Spike 2 closure doc |
| 01-v0.1 | 10-candi-cli | planned | |
| 01-v0.1 | 11-v01-release | planned | |
| 02-v0.2 | — | planned | phase README only |
| 03-v0.3 | — | planned | phase README only |
| 04-v0.4 | — | planned | phase README only |
| 05-v0.5 | — | planned | phase README only |
| 06-v0.9 | — | planned | phase README only |

## Merge history

| Date | From → to | Commit | Independent reviewer verdict |
|---|---|---|---|
| 2026-08-20 | slice/00-01-workspace-bootstrap → dev | 07ad909 | APPROVE after 2 rounds (round-1 nit fixed: TOML trailing newlines; round-2: CI fix commit 007e7bd) — workspace, toolchain pin 1.97.1, CI hashFiles fix + workflow-lint |
| 2026-08-20 | slice/00-02-ci-drill-gates → dev | 78ae626 | APPROVE — CI green twice, cache hit proven; 2 nits non-blocking (deny.toml trailing newline; AGPL-3.0 SPDX deprecation tracked for later normalization) |
| 2026-08-20 | slice/00-03-benchmark-harness → dev | e7c5461 | APPROVE — methodology verified vs spike probe, error paths live-tested; 2 nits non-blocking (run.sh:26 message cosmetics; bench glob noted for 01/05) |
| 2026-08-20 | slice/01-01-candi-pdf-trait → dev | 5b0d1c1 | APPROVE — verbatim conformance exact, 2 feature modes 23/23 tests; nit: trailing-newline pattern (3rd) tracked for prevention |
| 2026-08-20 | slice/01-02-mupdf-backend → dev | ad83c7b | APPROVE — mupdf-backend feature (mupdf 0.8.0); fz_error_code mapping; zero-page → Malformed guard with fixtures; independent reviewer sign-off |
| 2026-08-20 | slice/01-03-pdfium-backend → dev | b9902d0 | APPROVE — pdfium-render 0.8.37 (chromium/7543); FPDF error mapping + zero-page catalog sniff; CI libpdfium pin + PDFIUM_LIB; permissive matrix green |
| 2026-08-20 | slice/01-04-backend-parity-hardening → dev | 3d519af | APPROVE (round 2b after REQUEST CHANGES) — shared parity suite + text-layer sampling; feat 153cafc |
| 2026-08-20 | slice/01-05-benchmarks-both-backends → dev | c3634e0 | APPROVE — reader_peak budget gate; PR #10; feat fc5a2af |

**Phase 00 (foundations) complete** after the 00/03 merge.

## How to update

- **Before every merge** (slice → `dev`, or `dev` → `main`): set the slice status
  in-progress → committed → merged, and add a row to the merge history with the
  date, the merge direction, the commit hash, and the independent reviewer's
  verdict.
- When a slice starts: mark it **in-progress**. When its PR merges: **merged**.
