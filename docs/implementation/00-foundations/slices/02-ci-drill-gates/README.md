# Slice 00/02 — ci-drill-gates

## Goal

Extend the CI the base already ships — root `.github/workflows/ci.yml` with
guarded fmt/clippy/test + docs-links jobs, and `.github/dependabot.yml` — into
the full drill gate: the clippy feature-mode matrix and dependency auditing
(cargo-deny or cargo audit) once the workspace exists.

## Prep / thinking

- **What the base ships:** `ci.yml` with fmt, clippy (`-D warnings`), test, and
  docs-links jobs; the Rust jobs are guarded with `if: hashFiles('Cargo.toml') != ''`
  so the base PR is green-skipped and they activate automatically once the
  workspace lands (slice 00/01). `dependabot.yml`: cargo + github-actions,
  weekly, open-pull-requests-limit 5.
- **Job matrix:** Linux (ubuntu-latest) now; the Windows job is added at the v0.1
  tag (acceptance requires Windows) — shape the matrix so adding an OS is a
  one-line change.
- **Feature-mode matrix:** clippy, `cargo build --release`, `cargo test` in
  **both** feature modes once features exist — `default` (both backends) and the
  permissive build (`--no-default-features --features pdfium-backend`). Until
  features exist, single mode; the matrix is written to grow.
- **Drill enforcement split:** fmt/clippy/tests are CI gates; deslop and the
  merciless review are human gates (cannot run in CI) — the split is documented
  in docs/implementation/README.md §How we work; CI just runs the automatable
  three.
- **Dependency auditing** (security-in-development plan): cargo-deny (license +
  advisory checks) or cargo audit. Dependabot already covers dependency updates;
  this job covers licenses and advisories.
- **CI caching:** mupdf's vendored C build takes 5–10 min on first build — cache
  the cargo target dir, keyed on `Cargo.lock`.

## Files

- `.github/workflows/ci.yml` (extend)
- `.github/dependabot.yml` (extend if needed)
- anything else CI-side the drill still needs

## Implementation tasks

1. Feature-mode matrix in `ci.yml`: clippy/build/test run in both feature modes.
2. Add the cargo-deny (or cargo audit) job — licenses + advisories.
3. Cache `target/` keyed on `Cargo.lock`.
4. Push the slice branch; prove CI green on it.
5. Run the drill.

## Verification

- CI green on the slice branch — the workflow proves itself.
- Locally: the same commands pass.
- Drill stages 3–4: optimization (CI time, cache key correctness) + line-by-line
  review of `ci.yml`.

## Commit message

```
ci: extend drill gates to feature modes and dependency audit
```

## PR notes

- Merge target: `dev`.
- Reviewer: both feature modes wired (or matrix ready to grow); `-D warnings` is
  a hard gate; cache keyed on `Cargo.lock`; cargo-deny + dependabot present per
  the security-in-development plan.

## Risks

- First-run build time (mupdf vendored build) — the cargo cache is the mitigation;
  a wrong cache key silently costs minutes per run.
- Windows job — deliberately deferred to the v0.1 release slice; note it, don't
  build it now.