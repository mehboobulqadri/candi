# Phase 05 — v0.5 (Python API + Distribution)

## Goal

A thin Python API over candi-core plus distribution (workflow phases 6 + 7). Entry:
can start any time after phase 01 (core API stable enough); does not block phases
02–04.

## What gets built

- **Python API (v0.5a)** — PyO3 thin wrapper over candi-core
  (`Document`, `.page_count`, `.page(n).text`, `.search()`); logic stays in Rust.
  manylinux wheels via maturin in CI.
- **Distribution (v0.5b)** — PyPI publish; AUR (`candi`, `candi-git`) via a tested
  PKGBUILD; CachyOS inclusion; Debian/Ubuntu package; Windows installer.

## Exit criteria (workflow phases 6 + 7)

- `pip install candi` works locally from a test index; `Document("book.pdf")
  .search(...)` works.
- Installable via at least PyPI + AUR without manual build steps.
- Decision gate: v0.5a then v0.5b tagged; v1.0 candidate reassesses roadmap scope.

## Planned slice themes

Slices are detailed when the phase starts (per docs/implementation/README.md §slice
workflow). Themes:

- Spike 6 — Python binding shape
- PyO3 wrapper over candi-core (Document, page_count, page_text, search)
- maturin manylinux wheels in CI
- Spike 7 — packaging research per platform
- PyPI publish; AUR PKGBUILD (candi, candi-git); CachyOS inclusion
- Debian/Ubuntu package; Windows installer
- Release slices v0.5a + v0.5b