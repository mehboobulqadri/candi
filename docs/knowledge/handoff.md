# Handoff

State for the next agent. Written at close-out by skillsmith, read at session start.
Overwritten every session — this file is always current, never stale.

## Done

- **Slice 01/11 v01-release engineering merged to `dev`** (PR #16, https://github.com/mehboobulqadri/candi/pull/16): slice feat `d7ce832`, HEAD `74a241e`, merge `638f8bcbf97b7c89e76ff38778ab64e66c724432`, mergedAt 2026-08-20T21:48:06Z. Independent reviewer APPROVE. CI `32420211385` all green including Windows rust-checks. Windows matrix (ubuntu + windows × default + pdfium-only); pdfium-win-x64 `chromium/7543` (dll sha256 `6b963c2be9cacbaa0c0c7f4bf6d20d2fd16729ebdaa9989978b0f7b119c1c1cb`). MuPDF builds on Windows (not pdfium-only fallback). Linux-only: apt fontconfig, bench (`/proc` RSS). `acceptance.md` honest: dogfood not met; tag/`main` not done; real-books bench SKIPPED.
- **Phase 01 code complete on `dev`** — slices 01/01–01/11 (PRs #4–#16). CLI, TUI, core, backends, sidecar, search, release engineering all merged.

## In progress

Nothing active.

## Next

**User-authorized release gates only** — no further Phase 01 engineering slices until user explicitly authorizes:

1. **Dogfood gate** — daily-use evidence before tag (not met; documented in `acceptance.md`).
2. **Tag `v0.1`** — version 0.1.0 on `dev` after dogfood.
3. **`dev` → `main`** — phase merge after tag.

Mia must not proceed on any of the above without explicit user OK.

## Open questions

- **Unsupported PDF-feature row** in hardening table — deferred in 01/04 (factory gating only).
- **MuPDF store cap / PDFium-for-full-doc-search** — deferred (not blocking release gates).

## Optional (not next-required)

- PR #1 (dependabot) still OPEN — user did not authorize merge.

## Hazards (read before coding)

| Hazard | Detail |
|---|---|
| **Release gates** | Dogfood, tag, and `main` merge are user stop-gates — not engineering work. |
| **CLI exit codes** | `Args::try_parse()` failures exit **1** — same as runtime errors; distinct stderr text per kind. |
| **Windows CI** | Matrix catches platform clippy (`c_ulong` is u32 on Windows — avoid unnecessary `as u32`). |
| **Bench corpus** | Real books in gitignored `bench/corpus-local.toml` — never commit. |
