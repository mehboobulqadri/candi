# Slice 01/05 — benchmarks-both-backends

## Goal

Production benchmark numbers for **both** backends on the full corpus, checked
against the architecture.md §Cross-cutting performance budget, with the spike doc
updated to production numbers.

## Prep / thinking

- Harness from 00/03 now measuring real engines: `open_ms` (before the file read),
  page-text (per-page **mean**, not whole-doc), first search-result latency,
  process-level RSS (baseline → VmHWM peak → delta), best-of-2, timeout-wrapped,
  nonzero exit on error paths.
- **Targets** (architecture.md §Cross-cutting): startup-to-first-page < 300 ms;
  open ≤ 150 ms; page-text < 20 ms/page mean; first search result < 300 ms;
  next/prev < 50 ms; peak RSS (`reader_peak`, page window) < 200 MB. MuPDF
  silberschatz `full_pass_peak` ~295 MB (256 MiB decode store on a sequential
  all-pages sweep) is recorded but not gated — not the v0.1 reader path.
- **Corpus:** the spike's corpus (frankl, sysdesign, cleancode, silberschatz,
  attention, alice-scan — same set for comparability) via the gitignored manifest,
  plus committed fixtures.
- **Spike doc update:** spikes/results/spike-1-pdf-backend.md gets per-backend
  notes with production numbers + a pointer to where the harness lives.
- Benchmarks are a **phase gate** (workflow standing rule 2) — these numbers are
  the baseline every later phase re-runs against; a regression is a blocker.

## Files

- `crates/candi-pdf/benches/` (productionize the 00/03 skeleton: real engine
  wiring, both backends)
- `bench/run.sh` (add page-text mean + search metrics)
- `spikes/results/spike-1-pdf-backend.md` (production numbers section)

## Implementation tasks

1. Wire both engines into the harness; add the missing metrics (page-text mean,
   search first/next).
2. Full-corpus run on the dev machine; record best-of-2 numbers.
3. Compare against the budget table — any miss is a blocker: investigate (resource
   caps, laziness) before relaxing a target.
4. Update the spike doc's per-backend notes with production numbers.
5. Run the drill.

## Verification

- All budget targets met on the full corpus; numbers recorded in the spike doc.
- Fresh-machine fixture-only run stays green (CI).
- Drill: methodology honesty (best-of-2, no cherry-picking — the spike's review
  fixed these exact bugs); line-by-line.

## Commit message

```
bench(candi-pdf): benchmark both backends against v0.1 budget
```

## PR notes

- Merge target: `dev`.
- Reviewer: methodology matches the fixed probe exactly (open_ms before file read,
  honest RSS delta, best-of-2); numbers are production, not spike; targets met or
  blocked loudly with investigation.

## Risks

- MuPDF silberschatz `full_pass_peak` ~295 MB exceeds 200 MB but is expected
  (MuPDF `FZ_STORE_DEFAULT` during a full page sweep); v0.1 gates `reader_peak`
  (~44 MB). Per-page store purge was measured worse (~940 MB); no cheap
  `mupdf::Context` store cap in this slice.
- CI does not run the full corpus — the dev-machine run is the gate; CI stays
  fixture-only.