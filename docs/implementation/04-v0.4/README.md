# Phase 04 — v0.4 (Android)

## Goal

Port candi-gui to Android (project.md 4.4; architecture.md §Frontends). Entry:
phase 03 exit.

## What gets built

- Same Rust core — no separate document engine.
- Android file picker, "Open With" integration, touch navigation.
- Storage permissions handled per Android lifecycle constraints.

## Exit criteria (workflow Phase 5)

- APK builds, installs, and opens/reads a PDF on a real or emulated device.
- Decision gate: v0.4 tagged.

## Planned slice themes

Slices are detailed when the phase starts (per docs/implementation/README.md §slice
workflow). Themes:

- Android build wiring (cargo-ndk / the framework's Android toolchain)
- File picker + "Open With"
- Touch navigation (scroll, page turn)
- Storage permissions per the Android lifecycle
- Benchmark re-run + v0.4 release slice