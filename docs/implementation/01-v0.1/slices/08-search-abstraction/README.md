# Slice 01/08 — search-abstraction

## Goal

Core-level search: a lazy per-page scan over the backend with a result list and
next/prev cursor — the first result is cheap, the whole document is never
materialized.

## Prep / thinking

- `SearchSession { query, results: Vec<(page, offset)>, cursor }` over lazy
  `page_text` (architecture.md §candi-core). The result list holds page numbers
  (and offsets) only — **never page text**.
- **Scan policy:** pages extracted and scanned one at a time **on demand**. First
  result must be cheap (< 300 ms budget). Decide: search-forward-from-current-page
  with wrap, blocking scan that stops at the first result (v0.1 keeps it minimal;
  background/incremental scanning is a later phase).
- **Case sensitivity:** decide in prep (insensitive is friendlier) and pin it in
  the tests.
- **Offset semantics:** character offset within the page's text; the TUI scrolls to
  it. Pin the definition.
- **Cursor:** n/N move next/prev through `results`; wrap behavior decided in prep.
- Works against **any** `Document` impl — tests use a fake document (or the
  StubBackend), so core tests never need real PDFs (REQUIREMENTS.md NFR).

## Files

- `crates/candi-core/src/search.rs`
- `crates/candi-core/tests/search.rs`

## Implementation tasks

1. `SearchSession` API: `new(document, query)`, `next()`, `prev()`, `current()`,
   `results()`.
2. Lazy scan implementation: page-at-a-time, results cached, cursor with wrap.
3. Tests with a fake document: multi-page hits, first-result-cheap (scan stops
   early), laziness proven by a `page_text` call-count assertion (no whole-doc
   extraction), next/prev wrap.
4. Run the drill.

## Verification

- Unit tests as listed; the call-count assertion proves laziness.
- Drill stage 3: no whole-doc materialization, no unbounded memory (page numbers
  only); stage 4: line-by-line.

## Commit message

```
feat(candi-core): add lazy search abstraction
```

## PR notes

- Merge target: `dev`.
- Reviewer: laziness proven by tests (page_text call count), cursor wrap
  semantics, case policy pinned, no page text retained in results.

## Risks

- Case-insensitive matching across backends may differ on unicode — pin the
  matcher (simple lowercase-per-char, documented limitation) in prep.