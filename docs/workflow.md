---
title: Workflow
nav_order: 4
---

# Candi — Project Workflow (Start to End)

This defines the procedure for taking Candi from an idea to a distributed application. Each phase has entry criteria, activities, exit criteria, and a decision gate. Do not enter a phase until its entry criteria are met.

## Phase 0 — Foundations

**Entry:** none (starting point).

**Activities:**
- Finalize `project.md` requirements and non-goals
- Set up the Rust workspace skeleton (`candi-core`, `candi-pdf`, `candi-theme`, `candi-tui`, `candi-cli` as empty crates)
- Set up CI: build + test on every push, for Linux at minimum
- Choose a placeholder license (revisit after Spike 1)

**Exit criteria:** workspace builds an empty binary in CI.

**Decision gate:** none — proceed automatically.

---

## Phase 1 — Blocking Spikes

**Entry:** Phase 0 complete.

**Activities:**
- Execute Spike 1 (PDF backend + licensing) — see `spikes.md`
- Execute Spike 2 (text-first rendering quality) using the chosen backend
- Document findings and failure modes in `/spikes/results/`

**Exit criteria:**
- A PDF backend is chosen with a license compatible with Candi's intended distribution (PyPI, AUR, Debian)
- Structured text extraction is proven to work cleanly on single-column documents, with known/documented limitations for multi-column and footnotes

**Decision gate:** ⛔ Do not write `candi-pdf` production code until this gate is passed. If no backend passes the license check, stop and re-evaluate the project's own license before continuing.

---

## Phase 2 — Core & TUI (v0.1)

**Entry:** Phase 1 gate passed.

**Activities:**
- Implement `candi-pdf` backend integration behind a `Document Backend` trait
- Implement `candi-core`: page navigation, search abstraction, reading-position tracking
- Implement `candi-tui`: scrolling, pagination, search UI, default keybindings
- Implement `candi-cli`: `candi book.pdf`
- Stand up benchmarks (startup time, page nav latency, search latency, memory) against real-world test PDFs (novel, paper, large doc, scanned-with-text-layer doc) — created in this phase, run in every phase after
- Handle malformed/encrypted/scanned-without-text PDFs with explicit "unsupported" messaging, not silent failure

**Exit criteria:**
- `candi book.pdf` opens, displays readable text, scrolls, paginates, searches, and remembers position across restarts
- Benchmarks pass an internally agreed baseline (define target numbers before starting, not after)
- No crashes on the malformed-PDF test set

**Decision gate:** v0.1 is tagged and usable daily by at least one person (dogfooding) before moving on.

---

## Phase 3 — Reader State & Themes (v0.2)

**Entry:** Phase 2 gate passed, dogfooding underway.

**Activities:**
- Execute Spike 4 (sidecar file format) if not already resolved
- Implement `candi-theme`: semantic token schema, TOML loading, validation, built-in themes (light/dark)
- Implement warm/night mode as independent from dark mode
- Implement bookmarks and custom chapters, stored in the versioned sidecar file
- Implement TOC view merging PDF outline + custom chapters

**Exit criteria:**
- Users can bookmark, create chapters, and switch themes without editing the source PDF
- Sidecar schema is versioned and a mismatch is handled explicitly
- At least 2 built-in themes ship (light, dark)

**Decision gate:** v0.2 tagged. Re-run benchmarks from Phase 2 to confirm no regression.

---

## Phase 4 — GUI (v0.3)

**Entry:** Phase 3 gate passed. Spike 3 (GUI framework) can run in parallel with Phase 3 if resourcing allows.

**Activities:**
- Implement `candi-gui` on the chosen framework
- Open file (dialog, drag & drop, recent documents)
- Render actual PDF pages (not just text) — enables images/diagrams
- Reuse `candi-core` document API unchanged

**Exit criteria:**
- GUI opens the same documents as the TUI, sharing reading position and bookmarks via the same sidecar
- No document logic exists inside `candi-gui` that isn't in `candi-core`

**Decision gate:** v0.3 tagged.

---

## Phase 5 — Android (v0.4)

**Entry:** Phase 4 gate passed.

**Activities:**
- Port `candi-gui` to Android via the chosen framework's Android support
- Implement file picker, "Open With", touch navigation, storage permissions

**Exit criteria:** APK builds, installs, and opens/reads a PDF on a real or emulated device.

**Decision gate:** v0.4 tagged.

---

## Phase 6 — Python API (v0.5a)

**Entry:** can start any time after Phase 2 (core API is stable enough), does not block Phases 3–5.

**Activities:**
- Execute Spike 6 (Python binding shape)
- Implement PyO3 bindings, thin wrapper only
- Build manylinux wheels via maturin in CI

**Exit criteria:** `pip install candi` works locally from a test index; `Document("book.pdf").search(...)` works.

**Decision gate:** v0.5a tagged.

---

## Phase 7 — Distribution (v0.5b)

**Entry:** Phase 6 complete (or in progress), core feature set stable (post-Phase 4 at minimum).

**Activities:**
- Execute Spike 7 (packaging research) if not already done
- Publish to PyPI
- Submit AUR package (`candi`, `candi-git`) via a tested `PKGBUILD`
- Pursue CachyOS inclusion, following its repository procedures
- Build Debian/Ubuntu package, maintained and tested separately
- Windows installer

**Exit criteria:** Candi is installable via at least PyPI + AUR without manual build steps.

**Decision gate:** v1.0 candidate — reassess scope for what comes next (EPUB? Annotations? Advanced rendering?) rather than assuming the original roadmap order still applies.

---

## Phase 8 — Advanced Rendering (v0.9, ongoing)

**Entry:** v1.0 candidate shipped and stable.

**Activities:**
- Execute Spike 5 (terminal image protocols)
- Add optional Kitty/Sixel/Braille rendering backends to the TUI
- Advanced dark/warm PDF page rendering (contrast, brightness, warmth transforms), not just extracted-text theming

**Exit criteria:** defined per-feature; this phase is intentionally open-ended and lowest priority.

---

## Standing Rules (apply across every phase)

1. **No phase starts before its blocking spike is resolved.** A spike result is a written artifact, not a verbal conclusion.
2. **Benchmarks are re-run at the end of every phase**, not just once. A regression is a blocker for tagging that phase's release.
3. **Every new feature is checked against the project's design principle** before it's accepted: *does this make reading better without making Candi unnecessarily complicated?*
4. **The core never gains frontend-specific logic.** If TUI or GUI code needs something only it uses, it stays in `candi-tui`/`candi-gui`, not `candi-core`.
5. **License compatibility is checked whenever a new dependency is added**, not just at Spike 1.

## Milestone-to-Phase Map

| Phase | Version tag | Corresponds to original milestone |
|---|---|---|
| 0 | — | Setup |
| 1 | — | Research Roadmap (PDF section) |
| 2 | v0.1 | Milestone 1 + 2 (Core, TUI) |
| 3 | v0.2 | Milestone 3 + 4 (Reader state, Themes) |
| 4 | v0.3 | Milestone 5 (GUI) |
| 5 | v0.4 | Milestone 6 (Android) |
| 6 | v0.5a | Milestone 7 (Python) |
| 7 | v0.5b | Milestone 8 (Distribution) |
| 8 | v0.9 | Milestone 9 (Advanced rendering) |
