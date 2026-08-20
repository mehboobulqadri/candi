# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

Where we are: base commit merged and verified — slice/00-base (1d667ef, reviewed FIX-THEN-MERGE → APPROVE) → dev (07d02a8) → main (414252d); progress.md records it (e519732). Repo is local-only: remote https://github.com/{{github}}/candi exists but has never been pushed. Working tree clean on main.

## Done

- Repo cleanup; spike-1 (MuPDF/PDFium/Poppler) results committed at spikes/results/spike-1-pdf-backend.md — MuPDF default + PDFium behind one trait, Poppler rejected.
- Architecture + phase/slice plan in docs/implementation/ (00-foundations = 3 slices, 01-v0.1 = 11 slices, 02–06 README-only), workflow docs.
- CI (fmt, clippy -D warnings, test, docs-links + dependabot), LICENSE (AGPL-3.0), SECURITY.
- skills-lock.json provenance (35 entries) committed; scripts/sync-creds.sh identity templating.
- Skills restored to 35 (27 wrongly pruned earlier; 4 web-only permanently dropped: agent-browser, e2e-testing, frontend-design, vercel-react-best-practices).

## In progress

Nothing mid-flight. Everything committed; no open work items.

## Next

1. Execute docs/implementation/00-foundations/slices/01-workspace-bootstrap/README.md — first Phase 00 action: cargo workspace, rust-toolchain.toml (1.97.1), LICENSE.md (SPDX AGPL-3.0), crate skeletons.
2. Follow the slice workflow: branch slice/00-01-workspace-bootstrap off dev, one commit per slice step, 4-stage drill + independent reviewer (separate worktree), merge to dev only after both review rounds pass. Workflow: docs/implementation/README.md + .agents/skills/slice-workflow.
3. Local verification before any merge; update docs/implementation/progress.md before every merge.

## Open questions

- GH Pages deferred — revisit at v0.1+.
- Final PDF backend choice deferred — user decides later in v0.1.
- Remote never pushed — first push needs auth setup.
- Stale upstream ref `origin/master` noted (predates the remote being set up).
- Orphan-root artifact: dev/main first parent is pre-review root f4305ed (base amended into a new orphan root, merged with --allow-unrelated-histories -X theirs). Cosmetic; safe to regraft later if desired.
