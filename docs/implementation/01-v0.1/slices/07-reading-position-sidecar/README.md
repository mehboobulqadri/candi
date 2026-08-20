# Slice 01/07 — reading-position-sidecar

## Goal

Reading-position persistence: a versioned sidecar (`book.pdf.candi.toml`) with
atomic writes, tolerant of missing/corrupt state.

## Prep / thinking

- **Schema v1** (architecture.md):

  ```toml
  schema_version = 1
  [reading]
  page = 42
  scroll = 12
  updated_at = "2026-08-20T12:00:00Z"
  ```

  Sidecar = `<pdf>.candi.toml` next to the PDF; the source PDF is **never**
  modified.
- **Atomic writes:** temp file + rename in the same directory (rename is atomic on
  POSIX); decide the fsync policy (write + fsync file, then fsync the dir — keep
  minimal but safe).
- **Tolerant reads:** missing sidecar → `Ok(None)` (fresh start); corrupt /
  unparseable → fresh state **with a reported warning** (never silent data loss,
  never a crash); `schema_version` newer than supported → explicit error (fail
  loud — spikes.md Spike 4 requirement: version mismatches handled explicitly, not
  silently overwritten).
- **API shape:** `load(path) -> Result<Option<Position>>`, `save(path, position)` —
  called by the TUI on quit and on page change (architecture.md).
- **Concurrency:** last-write-wins for v0.1; Spike 4 (v0.2) owns locking /
  concurrent TUI+GUI access — note it, don't solve it.

## Files

- `crates/candi-core/Cargo.toml` (dep: toml)
- `crates/candi-core/src/state.rs`
- `crates/candi-core/tests/state.rs`

## Implementation tasks

1. `Position { page, scroll, updated_at }` + schema-v1 serde types.
2. `load()`: missing → `None`; corrupt → `None` + warning; newer schema → error.
3. `save()`: temp file + rename in the PDF's directory, atomic.
4. Tests: round-trip, missing, corrupt, newer-version, atomicity (no partial file
   on failure).
5. Run the drill.

## Verification

- Unit tests as listed; the atomicity test simulates a failure mid-write.
- Drill stage 3: no unbounded state, no retry loops, minimal write path;
  stage 4: line-by-line.

## Commit message

```
feat(candi-core): add versioned reading-position sidecar
```

## PR notes

- Merge target: `dev`.
- Reviewer: atomicity (temp + rename, same dir), each tolerant-read path has a
  defined behavior (missing / corrupt / newer schema), the PDF is untouched,
  `updated_at` format is pinned.

## Risks

- TOML crate choice (`toml` vs `toml_edit` — v0.1 only writes; plain `toml`
  suffices).
- fsync cost on page change — architecture.md says write on quit + page change;
  batching is a v0.2 concern.