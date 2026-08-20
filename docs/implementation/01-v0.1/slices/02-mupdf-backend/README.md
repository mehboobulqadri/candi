# Slice 01/02 — mupdf-backend

## Goal

The MuPDF engine behind the `mupdf-backend` feature: real open/extract on
fixtures, including the silent-open-on-truncated fix.

## Prep / thinking

- `mupdf 0.8.0` (vendored C build, `base14-fonts` feature — the spike's exact
  configuration; first build 5–10 min).
- **Ownership:** the `Document` owns its `fz_context` (open → drop); nothing
  leaks, nothing global (architecture.md, decided).
- **Error mapping** by `fz_error_code`, never by message text; unknown codes →
  `Other` (architecture.md §Contracts).
- **Passwords:** wrong vs missing distinguished via `authenticate()` (spike
  finding).
- **THE FIX** (spike finding, architecture.md §Security): a truncated/malformed
  file opens "successfully" as a 0-page doc — after open, `page_count() == 0` →
  `Error::Malformed("truncated or empty document")`, in the engine's open path.
- **Lazy:** open loads no text; `page_text(p)` extracts exactly page `p`;
  `page_count` cached at open, infallible.
- `NoTextLayer` detection is deliberately **not** here (slice 01/04 owns the
  sampling policy) — an empty page returns `Ok("")` per the contract.

## Files

- `crates/candi-pdf/Cargo.toml` (`mupdf-backend = ["dep:mupdf"]`)
- `crates/candi-pdf/src/backend/mod.rs`
- `crates/candi-pdf/src/backend/mupdf.rs`
- `crates/candi-pdf/tests/mupdf.rs` (fixture-based tests)

## Implementation tasks

1. Feature wiring: `mupdf-backend = ["dep:mupdf"]`.
2. `MupdfBackend`: `name()`, `open(path, password) -> Result<Box<dyn Document>>`.
3. Error mapping: `fz_error_code` → kinds (not-found → `NotFound`, permission →
   `PermissionDenied`, password → `Encrypted`/`WrongPassword` by whether a
   password was supplied, format → `Malformed`, else `Other`).
4. `Document` impl: cached `page_count`, lazy `page_text`, `page_positions` from
   `fz_stext` (blocks/lines/words per the pinned semantics).
5. Silent-open fix: `page_count() == 0` after open → `Malformed`.
6. Register in the factory; `available()` reflects the feature.
7. Fixture tests: attention paper opens with the right page count, first text page
   non-empty with a known snippet, truncated fixture → `Malformed`, encrypted dummy
   → `Encrypted`/`WrongPassword`.
8. Run the drill.

## Verification

- Tests above in default feature mode.
- Drill stage 2: fmt / clippy / deslop. Stage 3: laziness (no whole-doc
  extraction), no unbounded caches, engine lifetime (drop closes the context).
  Stage 4: line-by-line.
- Build check: default mode only (the pdfium feature does not exist yet).

## Commit message

```
feat(candi-pdf): add mupdf backend behind feature
```

## PR notes

- Merge target: `dev`.
- Reviewer: the 0-page → `Malformed` fix (the spike's reviewed bug class); mapping
  by error code, not message; ownership (context owned and dropped); laziness.

## Risks

- Vendored build time in CI — the cargo cache from slice 00/02 must cover it;
  first local build 5–10 min.
- The 294 MB RSS outlier on silberschatz is a slice 01/05 concern (lazy loading is
  the mitigation) — do not pre-optimize here.