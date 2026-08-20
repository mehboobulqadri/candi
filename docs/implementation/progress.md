# Candi — Implementation Progress

Current position: base docs merged to dev + main; phase 00 slices **planned** — next:
`00-foundations/01-workspace-bootstrap`.

## Status

| Phase | Slice | Status | Notes |
|---|---|---|---|
| base | docs (repo docs + spike results + this plan) | merged to dev + main | architecture, phase/slice plan, workflow, CI, LICENSE AGPL-3.0, SECURITY, spike-1 results, skills-lock.json provenance, sync-creds.sh |
| 00-foundations | 01-workspace-bootstrap | planned | |
| 00-foundations | 02-ci-drill-gates | planned | |
| 00-foundations | 03-benchmark-harness | planned | |
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
| 2026-08-20 | slice/00-base (`1d667ef`) → dev (`07d02a8`) → main (`414252d`) | `07d02a8`, `414252d` | APPROVE after FIX-THEN-MERGE round (6 findings fixed) — base contents: architecture, phase/slice plan, workflow, CI, LICENSE AGPL-3.0, SECURITY, spike-1 results, skills-lock.json provenance, sync-creds.sh |

## How to update

- **Before every merge** (slice → `dev`, or `dev` → `main`): set the slice status
  in-progress → committed → merged, and add a row to the merge history with the
  date, the merge direction, the commit hash, and the independent reviewer's
  verdict.
- When a slice starts: mark it **in-progress**. When its PR merges: **merged**.
