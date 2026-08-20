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
