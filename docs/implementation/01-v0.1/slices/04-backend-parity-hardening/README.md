# Slice 01/04 — backend-parity-hardening

## Goal

One shared behavioral suite running against **both** backends, pinning the
hardening table (malformed/encrypted/image-only → exact error kinds) and the
image-only detection policy.

## Prep / thinking

- **Parity harness pattern:** shared test functions take a backend factory;
  `tests/parity/{mupdf.rs,pdfium.rs}` instantiate them — identical assertions for
  both engines (the spike proved comparable behavior; this makes it a gate).
- **Hardening table** (expected kinds, identical for both backends — architecture.md
  §Security):

  | Case | Expected kind |
  |---|---|
  | missing file | `NotFound` |
  | unreadable path | `PermissionDenied` |
  | truncated / malformed PDF | `Malformed` (mupdf needs the 01/02 fix) |
  | encrypted, no password | `Encrypted` |
  | encrypted, wrong password | `WrongPassword` |
  | image-only / scanned | `NoTextLayer` (open-time first-page sampling, FR-003) |
  | unsupported PDF feature | `Unsupported` |
  | anything else | `Other` |

  No crash, no hang — every case timeout-wrapped, every case in CI.
- **Image-only detection policy (FR-003), decided here:** open-time sampling of the
  **first pages**; all sampled empty → `NoTextLayer` with the OCR-not-supported
  message. Sample size is a prep decision (e.g. first 3 pages) — must cover
  empty cover pages without scanning the whole doc, and must stay inside the
  150 ms open budget. Lives in a shared helper wired into both engines' open path
  (or a factory-level wrapper — decide; both engines must behave identically).
- **Wrong vs no password:** mupdf distinguishes via `authenticate()`; pdfium via
  `FPDF_ERR_PASSWORD` + whether a password was supplied — the suite pins both.
- **Fixtures:** truncated + image-only are generated (head -c; pdftoppm + magick)
  with hard-fail on missing tools; dummy-encrypted is committed.

## Files

- `crates/candi-pdf/tests/parity/mod.rs` (shared suite)
- `crates/candi-pdf/tests/parity/mupdf.rs`
- `crates/candi-pdf/tests/parity/pdfium.rs`
- `crates/candi-pdf/tests/common/fixtures.rs` (fixture paths + generation helpers)
- `crates/candi-pdf/src/textlayer.rs` (sampling helper — wherever the policy lands)

## Implementation tasks

1. Write the shared parity suite: page count, first-page snippet, positions sanity,
   the full hardening table.
2. Implement first-page sampling for `NoTextLayer`; wire into both engines' open
   path.
3. Fixture-generation helpers with hard-fail on missing tools.
4. Run the parity suite in **both** feature modes in CI (the 00/02 matrix covers
   it).
5. Run the drill.

## Verification

- Parity suite green in default AND permissive modes.
- Hardening table: every row asserted by an automated test, timeout-wrapped.
- Drill stage 3: sampling is bounded (first pages only), no whole-doc scans;
  stage 4: line-by-line.

## Commit message

```
test(candi-pdf): add backend parity suite and hardening matrix
```

## PR notes

- Merge target: `dev`.
- Reviewer: identical expectations across engines — no per-backend special-casing
  of the table; `NoTextLayer` sampling bounded; every hardening case automated and
  timeout-wrapped; CI runs both modes.

## Risks

- Fixture-generation tool availability (magick/pdftoppm) in CI — generate once,
  cache artifacts.
- Sample-size tuning: too few pages misclassifies image-heavy books with late text;
  too many slows open. The 150 ms open budget bounds the sample.