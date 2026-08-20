# Slice 00/03 — benchmark-harness

## Goal

The production benchmark harness — the spike's methodology with no benchmark
dependencies — plus the corpus manifest, ready to measure candi-pdf in phase 01.

## Prep / thinking

- **Harness structure:** a bench binary inside `crates/candi-pdf` using `std::time`
  only, driven by a `run.sh`. Methodology fixed from the review-fixed spike probe:
  `open_ms` timed before the file read, best-of-2, process-level RSS (VmRSS
  baseline → VmHWM peak → delta), every run timeout-wrapped, nonzero exit on error
  paths.
- **Corpus references:** committed small fixtures (attention paper, alice-scan,
  dummy-encrypted — currently under `spikes/corpus/`) + local real books
  referenced by a **gitignored** manifest (paths live on the dev machine only,
  never in the repo). Real books are never
  committed. DECIDE: `spikes/corpus/` is currently ignored by `.gitignore`
  (`/spikes/*` except `results/`) — un-ignore the fixtures or move them into the
  workspace; the manifest must fail loudly if a listed book is missing.
- **Fixture generation:** truncated (`head -c`), image-only (pdftoppm → magick),
  encrypted (committed dummy) — generated in-harness exactly like the spike's
  `run.sh`, hard-failing with a clear message if ImageMagick/pdftoppm are missing,
  so a fresh machine never silently tests not-found instead.
- **Output columns** match the spike table (open_ms, extract/page-text ms, chars/s,
  RSS baseline/peak/delta) so numbers stay comparable across time.

## Files

- `crates/candi-pdf/benches/bench.rs` (bench binary, std timing only)
- `bench/run.sh` (corpus loading, fixture generation, best-of-2, timeout wrapping)
- `bench/corpus.toml` (committed fixtures) + `bench/corpus-local.toml` (gitignored
  local books)
- `.gitignore` updates: the local manifest + generated fixtures
- `spikes/corpus/` — un-ignored if the fixtures move decision says so

## Implementation tasks

1. Bench binary with the fixed methodology (std timing; RSS via
   `/proc/self/status`).
2. `run.sh`: manifest loading, fixture generation, best-of-2, timeout wrapping,
   nonzero exits, one-corpus-match-per-book assertion.
3. Resolve the `spikes/corpus/` ignore question (commit fixtures or move them).
4. Gitignored local manifest listing the spike's corpus books.
5. Fresh-machine run (fixtures only) green; full-corpus run on the dev machine.
6. Run the drill.

## Verification

- Fresh-machine run: fixtures only, all generated files created, no silent
  not-found passes.
- Full-corpus run on the dev machine completes within the timeout budget.
- Drill: static analysis (bench code), performance review (methodology honesty —
  the spike's review fixed exact-measurement bugs; do not reintroduce them),
  line-by-line.

## Commit message

```
bench: add benchmark harness and corpus manifest
```

## PR notes

- Merge target: `dev`.
- Reviewer: methodology matches the fixed spike probe (open_ms before file read,
  honest process peak + delta, best-of-2); fixtures committed but local books
  never; every error path exits nonzero.

## Risks

- ImageMagick/pdftoppm missing on fresh machines — hard-fail with a clear message
  (spike behavior).
- Local corpus paths are machine-specific — manifest gitignored by design; CI runs
  fixtures only.
- RSS via `/proc/self/status` is Linux-only — the Windows job (v0.1 tag) needs a
  fallback; note it, don't build it now.