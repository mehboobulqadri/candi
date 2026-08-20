# Phase 00 — Foundations

## Goal

An empty-but-building workspace, CI that enforces the automatable drill stages, and
the benchmark harness + corpus every later phase reuses. Entry: none (starting
point). Exit: workspace builds an empty binary in CI; the drill passes on the
skeleton; CI green (workflow Phase 0).

## What gets built

- **Cargo workspace** with the five v0.1 crates as empty skeletons
  (`candi-core`, `candi-pdf`, `candi-theme`, `candi-tui`, `candi-cli`) per
  project.md §5; AGPL-3.0 LICENSE.md at repo root. `candi-gui` and `bindings/python`
  are created when their phases start.
- **GitHub Actions CI** — fmt, clippy `-D warnings`, release build, tests; Linux
  now, Windows job at the v0.1 tag. Dependency auditing (cargo-deny + dependabot).
- **Benchmark harness** — the spike's methodology productionized: std timing,
  best-of-2, process-level RSS, corpus manifest (committed fixtures + local
  books), generated error fixtures.

## Exit criteria

- Workspace builds an empty binary in CI (workflow Phase 0 exit criterion).
- Drill stages 1–4 pass on the skeleton; CI green on `dev`.
- Benchmark harness runs on a fresh machine (fixtures only) and on the dev machine
  (full corpus).

## Slice index

| # | Slice | Status |
|---|---|---|
| 01 | workspace-bootstrap | planned |
| 02 | ci-drill-gates | planned |
| 03 | benchmark-harness | planned |

Slice READMEs: `slices/01-workspace-bootstrap/`,
`slices/02-ci-drill-gates/`, `slices/03-benchmark-harness/`.