# Slice 01/10 — candi-cli

## Goal

The CLI entry: `candi book.pdf [--backend mupdf|pdfium]`, the open flow, explicit
error UX, and reading-position resume.

## Prep / thinking

- clap. Positional file + `--backend` (default mupdf; unknown value → explicit
  `Unsupported` error — architecture.md §candi-pdf). No args → usage.
- **Open flow:** parse → factory `open` → sidecar `load` → position resume (jump
  to the saved page) → launch the TUI. A `--text` mode is **not** in v0.1 scope —
  the TUI is the interface (project.md 4.1).
- **Error UX:** every failure maps to a kind → explicit message, nonzero exit
  (FR-010, cross-cutting error philosophy). Encrypted / image-only / malformed get
  their dedicated messages — never a silent empty page.
- **Exit codes:** decide granularity in prep (1 for all errors with distinct stderr
  text per kind is acceptable for v0.1; a per-kind code table is an option — pick
  one and pin it).
- **Tests:** integration tests spawning the binary against fixtures (missing file
  → `NotFound` + nonzero; encrypted → `Encrypted`; image-only → `NoTextLayer`;
  unknown backend → `Unsupported`).

## Files

- `crates/candi-cli/Cargo.toml` (clap, candi-pdf, candi-core, candi-tui)
- `crates/candi-cli/src/main.rs`
- `crates/candi-cli/tests/cli.rs`

## Implementation tasks

1. Arg parsing (file, `--backend` with value validation).
2. Open flow wiring the factory + sidecar resume.
3. Error mapping to exit codes + messages.
4. Integration tests per fixture.
5. Run the drill.

## Verification

- Integration tests: each error path exits nonzero with the expected message; the
  happy path launches the TUI (headless CI: decide in prep — TestBackend covers
  rendering, or a test hook gates TUI startup).
- Drill stage 3: startup-path allocations (startup < 300 ms budget);
  stage 4: line-by-line.

## Commit message

```
feat(candi-cli): add CLI entry with error UX and position resume
```

## PR notes

- Merge target: `dev`.
- Reviewer: error UX explicit per kind (no silent failures), `--backend`
  validation, position resume works across restarts (FR-009), startup within
  budget.

## Risks

- Headless CI cannot run the TUI — the happy-path test needs the prep decision
  (TestBackend-driven via a test hook, or assert pre-TUI behavior only).