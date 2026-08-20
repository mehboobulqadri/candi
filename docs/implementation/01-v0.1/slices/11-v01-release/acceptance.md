# v0.1 acceptance evidence (engineering slice 01/11)

Workspace version: **0.1.0** (`Cargo.toml` workspace.package.version). **Not tagged**
— tag, GitHub release, and `dev` → `main` merge require explicit user authorization.

Date: 2026-08-21. Scope: evidence collection + Windows CI job. Dogfood and release
artifacts are **blocked on user**.

## Functional requirements

| ID | Requirement | Evidence | Status |
|---|---|---|---|
| FR-001 | Open PDF from CLI | `crates/candi-cli/tests/cli.rs` (headless open on `tiny.pdf`); `crates/candi-pdf/tests/{mupdf,pdfium,parity_*}`.rs open fixtures | Met (tests) |
| FR-002 | Validate input / recoverable errors | CLI: missing file, encrypted, unknown backend; PDF: truncated/malformed fixtures in parity + mupdf/pdfium tests (PR #5–#7) | Met (tests) |
| FR-003 | Detect text layer / image-only message | Open-time sampling (`candi-pdf/src/textlayer.rs`, slice 01/04); CLI `image_only_pdf_exits_with_no_text_layer_message` | Met (tests) |
| FR-004 | Extract text with positions | Backend unit + parity suites (`parity_mupdf.rs`, `parity_pdfium.rs`); `PagePositions` in trait tests | Met (tests) |
| FR-005 | Display text in terminal | TUI TestBackend (`crates/candi-tui/tests/ui.rs`, 14 tests); CLI `CANDI_NO_TUI=1` headless stdout | Met (tests) |
| FR-006 | Scroll | TUI `scroll_keys_change_offset_on_long_page`; core `ViewState` scroll in navigation tests | Met (tests) |
| FR-007 | Page navigation | TUI page/first/last keys; `candi-core/tests/navigation.rs` (16 tests, PR #11) | Met (tests) |
| FR-008 | Search | `candi-core/tests/search.rs` (16 tests, PR #13); TUI search next/prev/prompt tests | Met (tests) |
| FR-009 | Reading position sidecar | `candi-core/tests/state.rs` (11 tests, PR #12); CLI resume + corrupt/unsupported schema tests (PR #15) | Met (tests) |
| FR-010 | Graceful errors (no avoidable crashes) | CLI exit-1 UX (10 integration tests); parity error-path matrix; malformed/encrypted fixtures | Met (tests) |

## Platform

| Target | Evidence | Status |
|---|---|---|
| Linux | CI `rust-checks` on `ubuntu-latest` (both feature modes); local clippy/test green in slice worktree | Met (CI + local) |
| Windows | CI `rust-checks` on `windows-latest` (both feature modes) in PR #16 | **Job added** — first CI run failed on Windows-only clippy (`unnecessary_cast` in `pdfium.rs:502`); fix pushed; re-check PR checks |

### Windows backend note

First CI run (PR #16): **MuPDF vendored build succeeded** on `windows-latest`.
Failure was clippy `-D warnings` on `FPDF_GetLastError() as u32` (Windows `c_ulong`
is already `u32`). Fixed with `u32::try_from(...)`. Both default and pdfium-only
matrix entries compile MuPDF via workspace path deps (documented in `ci.yml`).

## Benchmarks

| Item | Result |
|---|---|
| Real books (frankl, sysdesign, …) | **SKIPPED** — no `bench/corpus-local.toml` in repo (same policy as Spike 2 / CI) |
| Fixtures-only re-run (2026-08-21, slice worktree) | `./bench/run.sh --fixtures-only`: mupdf + pdfium on `dummy-encrypted` error path only; budget gate off (`BENCH_CHECK_BUDGET=0`) |
| Production numbers | Prior full-corpus run in `spikes/results/spike-1-pdf-backend.md` §Production harness (2026-08-20) — **not overwritten** (no new real-book numbers this run) |

Fixtures-only excerpt:

```
mupdf    dummy-encrypted    Encrypted    ok    baseline 6 MB    reader peak 9 MB
pdfium   dummy-encrypted    Encrypted    ok    baseline 6 MB    reader peak 8 MB
```

## Licensing / distribution

| Backend | License | v0.1 posture |
|---|---|---|
| MuPDF | AGPL-3.0 | Default build; accepted per spike-1 decision |
| PDFium | BSD-3-Clause (lib) + MIT/Apache-2.0 (bindings) | Permissive build mode; prebuilt lib pinned in CI (chromium/7543) |

## Release gates (explicitly not done)

| Gate | Status |
|---|---|
| Dogfood (daily use by ≥1 person) | **Not met** — blocked on user |
| Git tag `v0.1` | **Not done** — blocked on user authorization |
| GitHub release | **Not done** |
| `dev` → `main` phase merge | **Not done** — separate authorized step |
| Full benchmark re-run on real books | **Blocked** without local corpus manifest |

## Related PRs (v0.1 feature stack on `dev`)

| Slice | PR |
|---|---|
| 01/05 benchmarks | #10 |
| 01/06 navigation | #11 |
| 01/07 sidecar | #12 |
| 01/08 search | #13 |
| 01/09 TUI | #14 |
| 01/10 CLI | #15 |
| 01/11 engineering (this slice) | pending |
