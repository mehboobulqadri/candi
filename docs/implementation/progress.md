# Candi — Implementation Progress

Current position: phase 00 (foundations) complete — slices 00/01, 00/02, 00/03
merged; next: `01-v0.1/01-candi-pdf-trait`.

## Status

| Phase | Slice | Status | Notes |
|---|---|---|---|
| base | docs (repo docs + spike results + this plan) | merged | lands as the repo's initial commit |
| 00-foundations | 01-workspace-bootstrap | merged | workspace skeleton + CI workflow fix (hashFiles job-level guard bug) |
| 00-foundations | 02-ci-drill-gates | merged | feature-mode matrix (grow-ready, single mode until features land), cargo-deny + deny.toml, target cache keyed on Cargo.lock |
| 00-foundations | 03-benchmark-harness | merged | bench harness (std-only bench binary + run.sh); committed fixture is dummy-encrypted only (real books local-only per repo policy); runtime-generated fixtures, corpus manifest, bench CI job |
| 01-v0.1 | 01-candi-pdf-trait | planned | |
| 01-v0.1 | 02-mupdf-backend | planned | |
| 01-v0.1 | 03-pdfium-backend | planned | |
| 01-v0.1 | 04-backend-parity-hardening | planned | |
| 01-v0.1 | 05-benchmarks-both-backends | planned | |
| 01-v0.1 | 06-candi-core-navigation | planned | |
| 01-v0.1 | 07-reading-position-sidecar | planned | |
| 01-v0.1 | 08-search-abstraction | planned | |
| 01-v0.1 | 09-candi-tui | planned | |
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

**Phase 00 (foundations) complete** after the 00/03 merge.

## How to update

- **Before every merge** (slice → `dev`, or `dev` → `main`): set the slice status
  in-progress → committed → merged, and add a row to the merge history with the
  date, the merge direction, the commit hash, and the independent reviewer's
  verdict.
- When a slice starts: mark it **in-progress**. When its PR merges: **merged**.
