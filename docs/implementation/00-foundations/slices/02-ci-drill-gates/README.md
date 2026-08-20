# Slice 00/02 — ci-drill-gates

## Goal

GitHub Actions CI that enforces every automatable stage of the drill — fmt,
clippy `-D warnings`, release build, tests — in both feature modes, on push and
PR. Plus dependency auditing (cargo-deny + dependabot).

## Prep / thinking

- **Job matrix:** Linux (ubuntu-latest) now; the Windows job is added at the v0.1
  tag (acceptance requires Windows) — shape the matrix so adding an OS is a
  one-line change.
- **Which checks in which mode:** fmt once (mode-independent); clippy, `cargo
  build --release`, `cargo test` in **both** feature modes once features exist —
  `default` (both backends) and the permissive build
  (`--no-default-features --features pdfium-backend`). Until features exist,
  single mode; the matrix is written to grow.
- **Drill enforcement split:** fmt/clippy/tests are CI gates; deslop and the
  merciless review are human gates (cannot run in CI) — the split is documented in
  docs/implementation/README.md §How we work; CI just runs the automatable three.
- **Dependency auditing** (security-in-development plan): cargo-deny (license +
  advisory checks) or cargo audit + dependabot, from day one.
- **CI caching:** mupdf's vendored C build takes 5–10 min on first build — cache
  the cargo target dir, keyed on `Cargo.lock`.

## Files

- `.github/workflows/ci.yml`
- `.github/dependabot.yml`

## Implementation tasks

1. `ci.yml`: on push + PR; toolchain from `rust-toolchain.toml`; `cargo fmt
   --check`; `cargo clippy --all-targets -- -D warnings`; `cargo build --release`;
   `cargo test` — both feature modes (single mode until features exist).
2. Add the cargo-deny check job (licenses + advisories).
3. `dependabot.yml`: weekly cargo updates.
4. Cache `target/` keyed on `Cargo.lock`.
5. Push the slice branch; prove CI green on it.
6. Run the drill.

## Verification

- CI green on the slice branch — the workflow proves itself.
- Locally: the same commands pass.
- Drill stages 3–4: optimization (CI time, cache key correctness) + line-by-line
  review of `ci.yml`.

## Commit message

```
ci: enforce drill gates in GitHub Actions
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