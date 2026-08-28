---
title: Implementation
nav_order: 6
---

# Candi — Implementation Plan & Working Method

Status: **planning complete.** Implementation is organized as **phases** — one phase
per product version (or foundations) — and each phase is delivered as small
**slices**: every slice is one independent, reviewable, mergeable commit with its
own README. Read only the phase + slice README you are executing — never
bulk-read this directory. Where we are right now: [progress.md](progress.md).

## Phase ↔ workflow mapping

| Phase (this dir) | Version | Workflow phase | Delivers |
|---|---|---|---|
| [00-foundations](00-foundations/README.md) | — | 0 (Setup) | workspace, CI gates, benchmark harness |
| [01-v0.1](01-v0.1/README.md) | v0.1 | 2 (Core & TUI) | candi-pdf, candi-core, candi-tui, candi-cli |
| [02-v0.2](02-v0.2/README.md) | v0.2 | 3 (Reader state & themes) | themes, bookmarks/chapters, TOC |
| [03-v0.3](03-v0.3/README.md) | v0.3 | 4 (GUI) | candi-gui |
| [04-v0.4](04-v0.4/README.md) | v0.4 | 5 (Android) | Android port |
| [05-v0.5](05-v0.5/README.md) | v0.5a+b | 6 + 7 (Python API + Distribution) | bindings, packaging |
| [06-v0.9](06-v0.9/README.md) | v0.9 | 8 (Advanced rendering) | terminal images, page rendering |

Workflow Phase 1 (spikes) is **COMPLETE** for its blocking decision (Spike 1) —
decision artifacts in
[spikes/results/spike-1-pdf-backend.md](../../spikes/results/spike-1-pdf-backend.md):
dual MuPDF + PDFium backend, AGPL-3.0 with a permissive-build escape hatch.
Spike 2 (text-first rendering quality) closes in phase 01, slice 09.

## How we work — the mandatory quality drill

Every task, no exceptions (the project's agent layer runs this drill; stated here
for the project):

1. **Organized implementation** — design-first: orient, plan, then a minimal,
   scoped diff; no unrelated refactors.
2. **Static analysis** — `cargo fmt --check`, `cargo clippy --all-targets -- -D
   warnings`, then the `deslop` pass (no unnecessary comments, defensive checks, or
   dead flexibility).
3. **Performance review** — hot paths, allocations, laziness; no unbounded caches,
   no module-level mutable state, no whole-document work where per-page suffices.
4. **Merciless line-by-line review** — an independent reviewer pass before any
   change is done; a change that fails review is fixed and re-reviewed, never merged
   on promise.
5. **Workflow/YAML changes** (`.github/workflows/`) are validated with actionlint
   before push — `hashFiles` is step-level only.

CI enforces the automatable stages (fmt, clippy, tests; the feature-mode matrix
lands with slice 00/02);
deslop and the merciless review are agent/human gates.

## The slice workflow (default for every slice)

Each slice = one commit: independent, reviewable, mergeable. Follow it in order.

1. **Prep / thinking** — resolve the slice README's design questions *before* writing
   code.
2. **Write** — minimal scoped implementation + tests.
3. **Drill reviews** — general review → optimization review → line-by-line review
   (the four stages above).
4. **Commit** — the exact conventional-commit message from the slice README.
5. **PR** — branch `slice/NN-name`, target `dev`. Local branch until remote auth is
   configured; push the PR when a remote exists.
6. **Independent review** — an independent reviewer re-reviews the PR in a **separate
   git worktree**: deterministic checks (fmt / clippy / tests in both feature modes)
   + optimization pass + line-by-line review.
   Local deterministic checks are the gate — if they pass locally, CI mirrors them
   and the PR is acceptable; CI exists to prove the same thing on GitHub, not to
   add new gates.
7. **Fix findings** — re-run the drill on the changes; the reviewer's verdict gates
   the merge.
8. **Merge** — slice to `dev`. After a major phase, `dev` merges to `main` with the
   **same** independent review procedure.
9. **Progress** — update [progress.md](progress.md) *before every merge*.

## Branch model

- `main` — stable; one merge per phase release (the phase merge).
- `dev` — integration; every slice merges here.
- `slice/NN-name` — short-lived per-slice branches (NN = slice number, name =
  slice dir name).

## Logging & bug tracking (plan)

- **Runtime diagnostics:** `tracing` — structured, leveled, spans per
  document/backend/page operation. No PII in logs.
- **Log location:** XDG state dir — `~/.local/state/candi/logs/`.
- **No telemetry.** Privacy default: nothing leaves the machine.
- **Bug tracking:** GitHub Issues with a bug-report template (version, backend,
  document, steps, logs). GitHub issues are **public** — no secrets or PII in
  reports.

## Security in development

- **SECURITY.md** at the repo root — vulnerability disclosure policy (see
  [SECURITY.md](../../SECURITY.md)).
- **Secrets hygiene:** local-only tooling, personal identity files, and any
  credentials live in gitignored paths; no secrets or credentials are ever
  committed.
- **CI dependency auditing:** cargo-deny (or cargo audit) + dependabot — lands in
  slice 00/02.
- **Application security** (untrusted PDFs, resource exhaustion, error handling):
  architecture.md §Security. Every change touching those paths gets a security
  review pass.

Current position and per-slice statuses: [progress.md](progress.md).
