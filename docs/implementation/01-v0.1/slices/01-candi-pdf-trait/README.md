# Slice 01/01 — candi-pdf-trait

## Goal

candi-pdf's public surface: the 8-kind Error enum, `Document`/`Backend` traits,
`PagePositions` types, factory + runtime selection, and a `StubBackend` for tests.
No real engines yet.

## Prep / thinking

- Copy the trait block from architecture.md §candi-pdf **verbatim** — Error kinds,
  `Document`/`Backend` signatures, and `PagePositions` types are pinned there.
- Error-mapping policy lives at the engine level; the kind taxonomy and the
  message-for-humans-only rule are pinned here — messages are never matched on
  (architecture.md §Contracts).
- `Result<Option<PagePositions>>` semantics: real errors propagate as `Err`;
  `Ok(None)` **only** when a backend has no positional API (the spike's
  `Option<Stats>` conflation was a reviewed bug — do not regress).
- **Factory:** `BackendKind { Mupdf, Pdfium }`, `available() -> compiled-in
  backends`, `open(kind, path, password)` with Mupdf default; unknown names →
  `Unsupported`. Declare the features now (`default = ["mupdf-backend",
  "pdfium-backend"]`, empty deps) so the registry is feature-aware from day one;
  `available()` lists only compiled-in backends.
- `StubBackend`: in-crate test double implementing the full trait with configurable
  behaviors (empty pages, scripted errors) — the parity suite (01/04) and core
  tests (01/06–08) build on it.

## Files

- `crates/candi-pdf/Cargo.toml` (features declared, no backend deps yet)
- `crates/candi-pdf/src/lib.rs` (re-exports)
- `crates/candi-pdf/src/error.rs`
- `crates/candi-pdf/src/factory.rs`
- `crates/candi-pdf/src/stub.rs` (StubBackend + unit tests)
- `crates/candi-pdf/tests/stub.rs` (trait-contract integration tests)

## Implementation tasks

1. Error enum with the eight kinds — `String` payloads except `NoTextLayer` (unit),
   matching architecture.md.
2. `PagePositions`/`Block`/`Line`/`Word` with the pinned block/line/word semantics.
3. `Document` + `Backend` traits, exact signatures.
4. Factory: `BackendKind`, `available()`, `open()` (Mupdf default; non-compiled
   backends → `Unsupported`).
5. `StubBackend` + tests: error kinds, empty page → `Ok("")`, cached `page_count`.
6. Run the drill.

## Verification

- Drill stage 2: fmt / clippy `-D warnings` / deslop.
- Unit + integration tests: trait contract (lazy `page_text`, infallible cached
  `page_count`, `Result<Option>` positions), error construction, factory selection,
  `Unsupported` for unknown names and missing features.
- Drill stage 3: no whole-document work anywhere (the contract forbids it), no
  module-level mutable state.
- Drill stage 4: independent line-by-line review.

## Commit message

```
feat(candi-pdf): add Document/Backend traits and error model
```

## PR notes

- Merge target: `dev`.
- Reviewer: signatures match architecture.md verbatim (pinned — a deviation
  ripples through every later slice); `Ok(None)` semantics; features declared even
  though empty; StubBackend covers every error kind.

## Risks

- Trait shape is pinned by the spike — re-opening it needs contradicting data; flag
  any felt need in the PR notes instead of silently changing.