---
title: Options & Spikes
nav_order: 3
---

# Candi — Options & Spike Plan

## Spike results

**Spike 1 is RESOLVED** — [full results](../spikes/results/spike-1-pdf-backend.md).

Outcome: Candi ships a dual, runtime-switchable PDF backend — MuPDF (default) and
PDFium behind one `Document` trait; Poppler is rejected (slowest, no structured API,
GPL). License: AGPL-3.0, with a permissive-build escape hatch (pdfium-only cargo
feature). Spikes 2–7 below are still open.

A spike is a short, throwaway investigation meant to answer one specific question, not to produce production code. Every spike below has a single pass/fail question attached to it. Do not proceed to the next dependent milestone until the blocking spikes for it are resolved.

## 1. Decision Points Overview

| # | Decision | Blocks | Priority |
|---|---|---|---|
| 1 | PDF backend + its license | Everything | Critical — do first |
| 2 | Text-extraction quality for TUI rendering | v0.1 TUI (phase 01, slice 09) | Critical |
| 3 | GUI framework (Slint vs alternatives) | v0.3 | High, but not urgent |
| 4 | Sidecar file format & schema versioning | v0.2 | Medium |
| 5 | Terminal image protocol(s) | v0.9 (advanced rendering) | Low |
| 6 | Python binding approach | v0.5 | Low |
| 7 | Packaging pipeline per platform | v0.5+ | Low, but research early to avoid surprises |

## 2. Spike 1 — PDF Backend & Licensing

**Question to answer:** which PDF engine gives the best text-extraction quality and performance for a license Candi can actually ship under?

| Option | Text extraction | Performance | License | Notes |
|---|---|---|---|---|
| MuPDF | Strong, structured text API | Very fast | AGPL-3.0 (or paid commercial license from Artifex) | AGPL likely forces Candi and its Python bindings to also be AGPL, or requires a commercial license — resolve before adopting |
| PDFium | Good | Fast | BSD-3-Clause (Google, from Chromium) | Permissive; less mature Rust bindings, may need FFI work |
| Poppler | Good | Moderate | GPL-2.0 | Same copyleft concern as MuPDF, generally less performant |

**Spike tasks:**

1. Confirm current licensing terms directly from each project's official docs (licenses have changed before; do not rely on memory).
2. Prototype text extraction with each candidate on 3 real documents: a novel, a two-column academic paper, a large scanned-but-has-text-layer document.
3. Compare: extraction fidelity (reading order, paragraph detection), raw performance (open + extract time), and Rust binding maturity.

**Pass/fail:** a backend is viable only if its license is compatible with Candi's intended license *and* it produces usable structured text on the two-column paper without manual reordering logic.

**Recommended order:** license check first (cheap, fast, potentially disqualifying) → then only prototype the licensable options.

## 3. Spike 2 — Text-First Rendering Quality

**Question to answer:** can structured text extraction (from the chosen PDF backend) be turned into "excellent reading" text without becoming a full layout-analysis project?

PDFs have no guaranteed semantic structure — headings, paragraphs, and reading order are visual conventions, not tagged data (unless the PDF is a "Tagged PDF," which most aren't).

**Spike tasks:**

1. Run structured-text extraction against: a single-column novel, a two-column paper, a document with footnotes, a document with a floating figure/caption.
2. Attempt heuristic reconstruction of reading order and paragraph breaks from position/font data.
3. Identify the failure modes (e.g. footnotes interleaved mid-paragraph, multi-column text merged wrong).

**Pass/fail:** single-column documents must render cleanly with minimal heuristics. Multi-column and footnote handling can be "good enough with known limitations" for v0.1 — but the limitations must be documented, not silently wrong.

## 4. Spike 3 — GUI Framework

**Question to answer:** does Slint meet the cross-platform (Linux/Windows/Android) and performance bar, or is a different Rust-compatible framework a better fit?

| Option | Cross-platform | Maturity | Notes |
|---|---|---|---|
| Slint | Linux, Windows, Android, embedded | Growing, commercial backing | Explicit candidate in original scope |
| egui | Linux, Windows, limited Android | Mature, immediate-mode | Simpler, may be weaker for touch/complex layouts |
| Tauri (web-based UI) | Broad | Mature | Pulls in a webview; conflicts with "minimal/native" goal |

**Spike tasks:**

1. Build a throwaway Slint app that renders a static page image and handles basic scroll/zoom, on Linux desktop.
2. Evaluate Android build story specifically (this is usually where frameworks fall down).
3. Only if Slint fails the Android check, evaluate egui as a fallback.

**Pass/fail:** framework must build and run touch-capable UI on Android without a separate rendering engine.

## 5. Spike 4 — Sidecar File Format

**Question to answer:** what schema for `book.pdf.candi.toml` supports chapters, bookmarks, reading position, and notes, while being safe under concurrent TUI+GUI access?

**Spike tasks:**

1. Draft a TOML schema with an explicit `schema_version` field from day one.
2. Test simultaneous read/write from two processes (TUI open + GUI open on the same file) and decide on a locking or last-write-wins strategy.
3. Decide what happens when a sidecar's `schema_version` is newer than the running Candi version (fail loud, not silent data loss).

**Pass/fail:** concurrent access must not corrupt the file, and version mismatches must be handled explicitly.

## 6. Spike 5 — Terminal Image Protocols (Deferred)

**Question to answer:** which terminal graphics protocol(s) are worth supporting for optional image rendering in the TUI.

Options: Kitty graphics protocol, Sixel, iTerm2 image protocol, Unicode/Braille/half-block fallback for terminals with no graphics support.

**Spike tasks:** low priority — only start after v0.2 ships. Survey terminal support coverage before investing; this is explicitly optional per the project's own design principle.

## 7. Spike 6 — Python Binding Shape

**Question to answer:** what's the minimum PyO3/maturin API surface that makes Candi useful as a library without leaking Rust implementation details.

**Spike tasks:** prototype `Document`, `.page_count`, `.page(n).text`, `.search()` against the chosen PDF backend; confirm manylinux wheel build works in GitHub Actions before committing to the API shape.

## 8. Spike 7 — Packaging Pipeline

**Question to answer:** what does a minimal, automatable release pipeline look like across PyPI, AUR, CachyOS, and Debian/Ubuntu.

**Spike tasks:** research each target's submission process (PKGBUILD for AUR, debian/ directory structure, CachyOS package request flow) early enough to shape CI, but don't build the pipeline until there's a v0.1 binary worth shipping.

## 9. Spike Execution Order

```
1. PDF backend licensing check        (blocks everything — days, not weeks)
2. PDF backend text-extraction spike  (blocks v0.1 core)
3. Text-first rendering spike         (blocks v0.1 TUI; lands in phase 01, slice 09)
        │
        ▼
   v0.1 build begins
        │
4. Sidecar file format spike          (blocks v0.2)
5. GUI framework spike                (blocks v0.3, can run in parallel with v0.1 build)
        │
6. Python binding spike               (blocks v0.5, low urgency)
7. Packaging pipeline research        (blocks v0.5, low urgency, research-only)
8. Terminal image protocol survey     (blocks v0.9, lowest priority)
```
