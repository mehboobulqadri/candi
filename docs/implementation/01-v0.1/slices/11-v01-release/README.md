# Slice 01/11 — v01-release

## Goal

The v0.1 release: full benchmark re-run, acceptance check against REQUIREMENTS.md,
dogfood gate, tag v0.1, and the `dev` → `main` phase merge with the independent
review procedure.

## Prep / thinking

- **Acceptance:** REQUIREMENTS.md §v0.1 acceptance criteria, one by one — opens
  representative PDFs, displays text, navigates, scrolls, searches, persists
  position, clearly reports image-only PDFs, no crashes on the malformed set,
  Linux + Windows, benchmark results, backend licensing compatible with the
  distribution strategy.
- **Benchmarks re-run** on both backends (workflow standing rule 2) — a regression
  is a blocker for the tag.
- **Windows:** verified at tag time via CI job or local build (the Windows job
  deferred from slice 00/02 lands here).
- **Dogfood gate** (workflow Phase 2 decision gate): v0.1 is **not** tagged until
  it is used daily by at least one person — evidence goes in the release notes.
- **`dev` → `main`:** the same independent review procedure as a slice PR — separate
  worktree, deterministic checks (both feature modes), optimization pass,
  line-by-line. The phase diff is the whole v0.1 feature set.
- **Release artifacts:** tag `v0.1`, release notes (features, benchmark table,
  known limitations incl. Spike 2), progress.md + handoff.md updated.

## Files

- `.github/workflows/ci.yml` (add the Windows job)
- `Cargo.toml` version bumps (0.1.0)
- `spikes/results/spike-1-pdf-backend.md` (final production numbers)
- tag `v0.1` + GitHub release notes

## Implementation tasks

1. Full benchmark re-run; every target met (or blocked loudly with
   investigation).
2. Acceptance checklist run (REQUIREMENTS.md); dogfood evidence collected.
3. Windows verification.
4. Version 0.1.0 + tag.
5. `dev` → `main` with the independent review.
6. progress.md + handoff.md updated.

## Verification

- The acceptance checklist itself (all boxes true, evidence linked); final
  benchmark table vs the architecture.md budget.
- The drill applies to the release diff (version bumps, CI job, docs).

## Commit message

```
release: v0.1
```

## PR notes

- Merge target: `main` — this is the phase merge (the one `dev` → `main` merge per
  phase, reviewed like every slice).
- Reviewer (independent): acceptance evidence, benchmark table vs budget, the
  dogfood gate is true (not aspirational), the release diff is minimal.

## Risks

- Dogfood gate timing — cannot be forced; the phase slips until true (by design).
- Windows verification may surface cross-platform bugs — timebox, fix, re-verify.