# Phase 02 — v0.2 (Reader State & Themes)

## Goal

Reader state and theming (workflow Phase 3): bookmarks, custom chapters, TOC, and
the theme engine. Entry: phase 01 exit (dogfooding underway).

## What gets built

- **candi-theme** — semantic token schema, TOML load + validation, safe fallback
  with a reported error, built-in themes (light, dark, warm).
- **Sidecar v2** — bookmarks + custom chapters added to the versioned sidecar
  (schema migration from v1).
- **TOC view** — PDF outline merged with custom chapters.
- **Config** — XDG `config.toml` with the backend-selection key (CLI flag
  precedence: flag > file key).

## Exit criteria (workflow Phase 3)

- Users can bookmark, create chapters, and switch themes without editing the
  source PDF.
- Sidecar schema is versioned; a version mismatch is handled explicitly.
- At least 2 built-in themes ship (light, dark).
- Decision gate: v0.2 tagged; Phase 2 benchmarks re-run with no regression.

## Planned slice themes

Slices are detailed when the phase starts (per docs/implementation/README.md §slice
workflow). Themes:

- Spike 4 first — sidecar format + concurrent TUI/GUI access (locking or
  last-write-wins decision; schema v2)
- candi-theme crate: token schema, validation, safe fallback, built-in themes
- Sidecar v2: bookmarks + custom chapters, v1 → v2 migration
- TOC view: PDF outline + custom chapters merged
- Config: backend-selection key with flag precedence
- Benchmark re-run + v0.2 release slice (with the same drill and review)