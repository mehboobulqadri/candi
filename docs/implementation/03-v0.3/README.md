# Phase 03 — v0.3 (GUI)

## Goal

candi-gui on the Spike 3-chosen framework — Slint is the first candidate; Android
portability is the gate (architecture.md §Frontends). Entry: phase 02 exit; Spike 3
can run in parallel with phase 02.

## What gets built

- candi-gui on the chosen framework, rendering **real PDF pages** (images,
  diagrams) — not just extracted text.
- Open file: dialog, drag & drop, recent documents.
- Shares candi-core's API + sidecar with the TUI — zero document logic in the GUI
  (architecture.md, standing rule 4).

## Exit criteria (workflow Phase 4)

- GUI opens the same documents as the TUI, sharing reading position and bookmarks
  via the same sidecar.
- No document logic inside candi-gui that isn't in candi-core.
- Decision gate: v0.3 tagged; benchmarks re-run.

## Planned slice themes

Slices are detailed when the phase starts (per docs/implementation/README.md §slice
workflow). Themes:

- Spike 3 — GUI framework decision (Slint vs egui fallback; Android check first)
- candi-gui skeleton on the chosen framework
- Real page rendering (images, diagrams)
- Open dialog / drag & drop / recent documents
- Sidecar + core-API sharing with the TUI (concurrent access per Spike 4)
- Benchmark re-run + v0.3 release slice