# Slice 01/03 — pdfium-backend

## Goal

The PDFium engine behind the `pdfium-backend` feature: Arc-shared engine from the
factory, `FPDF_ERR_*` mapping, `u16::try_from` page-index guards. The permissive
build (`--no-default-features --features pdfium-backend`) must compile and pass.

## Prep / thinking

- `pdfium-render 0.8.37`, `default-features off`, `pdfium_latest` + `thread_safe`
  (the spike's exact configuration).
- **Engine ownership** (architecture.md, decided): the backend factory creates
  `Arc<Pdfium>` once (dlopens `libpdfium`) and holds it outside documents; each
  `Document` holds the `Arc` + its `FPDF_DOCUMENT`, closed on drop — **nothing
  leaks** (the spike's `Box::leak` was a probe-only hack; production leaks
  nothing).
- **libpdfium resolution:** env override (`PDFIUM_LIB`) + executable-adjacent
  fallback for release builds; dlopen failure → explicit error at open, never a
  panic.
- **Error mapping:** `FPDF_ERR_*` → kinds — `FORMAT` → `Malformed`, `PASSWORD` →
  `Encrypted` (no password supplied) / `WrongPassword` (supplied), `FILE` →
  `NotFound`/`PermissionDenied`, unknown → `Other`.
- **`u16::try_from`** for every page index crossing into the engine (the spike's
  reviewed fix — pdfium's page type is `u16`; a huge document must not panic).
- **Positions:** approximated from page text objects/segments, normalized to the
  trait semantics (blocks ≈ text objects, lines ≈ segments). Counts will differ
  from MuPDF (67 vs 15 blocks on one page); semantics hold (architecture.md).
- **Permissive build** = the license escape hatch — it must compile and pass its
  tests, with zero AGPL third-party code linked.

## Files

- `crates/candi-pdf/Cargo.toml` (`pdfium-backend = ["dep:pdfium-render"]`)
- `crates/candi-pdf/src/backend/pdfium.rs`
- `crates/candi-pdf/tests/pdfium.rs`

## Implementation tasks

1. Feature wiring: `pdfium-backend = ["dep:pdfium-render"]`.
2. Factory: one-time engine creation (no module-level mutable state beyond a single
   initialization), `Arc<Pdfium>` shared across documents.
3. `PdfiumDocument`: owns `FPDF_DOCUMENT` (closed on drop), cached `page_count`,
   lazy `page_text`, positions approximation.
4. Error mapping per the `FPDF_ERR_*` table.
5. `u16::try_from` guards on every page-index boundary.
6. Fixture tests mirroring the mupdf slice — identical expectations (attention,
   truncated → `Malformed` at open, encrypted → kinds).
7. Permissive build: `cargo build --release --no-default-features --features
   pdfium-backend` + its tests.
8. Run the drill.

## Verification

- Same fixture tests as 01/02 with identical expectations on real docs (the parity
  suite formalizes this in 01/04).
- Permissive-mode build + tests green.
- Drill stage 3: engine created once, `Arc` shared, no per-document reload;
  stage 4: drop paths and `u16` guards line-by-line.

## Commit message

```
feat(candi-pdf): add pdfium backend behind feature
```

## PR notes

- Merge target: `dev`.
- Reviewer: Arc engine ownership (no leaks), `FPDF_DOCUMENT` drop path, `u16`
  guards, the permissive build actually compiles and passes, dlopen failure is an
  explicit error.

## Risks

- Prebuilt `libpdfium.so` download (7.6 MB, pinned 153.0.8009.0) — CI must fetch it
  (cached) and a checksum pins it.
- musl/static builds: pdfium is dlopen'd — static builds need the lib shipped next
  to the binary; note it, don't solve (v0.5 distribution).
- pdfium-render API churn — pin the version.