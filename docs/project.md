---
title: Project
nav_order: 1
---

# Candi — Project Description & Requirements

## 1. What Candi Is

Candi is a minimal, fast, cross-platform document reader with two frontends — a terminal UI (TUI) and a graphical UI (GUI) — sharing one native Rust core.

**Mission statement:** make reading documents fast, comfortable, and distraction-free.

Candi is not trying to be a PDF editor, an annotation powerhouse, or a document management system. It is a reader first, and every feature is evaluated against one question: *does this make reading better without making Candi unnecessarily complicated?*

## 2. Product Goals

| Goal | Meaning |
|---|---|
| Minimal | Small interface, few dependencies, no feature bloat |
| Fast | Instant-feeling navigation, low overhead, benchmarked from day one |
| Keyboard-first | TUI is a first-class interface, not a stripped-down GUI |
| Cross-platform | Linux, Windows, Android at launch; macOS later |
| Dual-interface | TUI and GUI both sit on the same core, neither is "primary" |
| Themeable | Fully custom themes without recompiling |
| Extensible | Images, diagrams, EPUB, annotations added progressively, not day one |
| Native | No Python required to run the app |
| Scriptable | A thin Python API wraps the native core |
| Distributable | PyPI, AUR, CachyOS, Debian/Ubuntu, eventually Windows/Android packages |

## 3. Non-Goals (v0.1 and beyond, unless explicitly revisited)

- Pixel-perfect PDF rendering in the TUI
- PDF editing or form-filling
- Annotation/markup tools
- Cloud sync
- Multi-format support (EPUB, MOBI, etc.) before PDF is excellent
- Plugin/extension system
- JavaScript execution inside PDFs (explicitly excluded for security)

## 4. Requirements

### 4.1 v0.1 — Minimum Viable Reader

This is intentionally smaller than the original scope draft. Chapters, bookmarks, and theming are valuable but not "minimum viable" — they go to v0.2 so v0.1 can ship fast and prove the core engine works.

**Functional requirements:**

- Open a PDF from the command line: `candi book.pdf`
- Extract text from PDF pages via a pluggable document backend
- Display extracted text in the TUI with readable formatting (paragraphs, basic spacing, page boundaries)
- Scroll and paginate through the document
- Jump to first/last page
- Full-text search within the document, with next/previous result navigation
- Remember the last reading position per document, across sessions
- Minimal default keybindings (see Section 6)

**Non-functional requirements:**

- Native binary; no Python runtime required
- Startup time and page-navigation latency benchmarked against real-world PDFs (books, papers, large docs) from the first working prototype
- Malformed/corrupted PDFs fail gracefully (no crash, no hang)
- Works over SSH (TUI has no dependency on local display/GPU)

**Explicitly out of scope for v0.1:** GUI, images/diagrams, custom chapters, bookmarks, dark/warm mode, configurable themes, Android, Python bindings, packaging beyond a local build.

### 4.2 v0.2 — Reader State & Themes

- Bookmarks
- Custom chapters, independent of the PDF's own table of contents
- Reader metadata stored in a sidecar file (`book.pdf.candi.toml`), never modifying the source PDF
- Sidecar schema is versioned from the first release of this feature
- Built-in dark mode and warm/night mode
- User-defined themes (TOML), built on semantic tokens (`background`, `foreground`, `heading`, `accent`, etc.), not widget-specific keys
- Table of contents view (from PDF outline, merged with custom chapters)

### 4.3 v0.3 — GUI

- Open file (dialog, drag & drop, recent documents)
- Renders actual PDF pages, not just extracted text (enables images, diagrams, equations, vector graphics)
- Same core document API as the TUI — no GUI-only document logic
- Desktop targets: Linux, Windows

### 4.4 v0.4 — Android

- Same Rust core, no separate document engine
- Android file picker, "Open With" integration
- Touch navigation
- Storage permissions handled per Android lifecycle constraints

### 4.5 v0.5 — Python API & Distribution

- Thin PyO3-based Python API (`Document`, `page()`, `search()`, etc.) — logic stays in Rust
- Packaging: PyPI (via maturin), AUR, CachyOS, Debian/Ubuntu, Windows installer

### 4.6 Cross-Cutting Requirements (apply to every phase)

- **Licensing:** the PDF backend's license must be resolved *before* it is adopted — see the Spikes document. This gates Candi's own license choice.
- **Security:** untrusted PDFs must not execute embedded JavaScript, must not allow arbitrary file access, and must be resilient to decompression bombs and malformed structures.
- **Accessibility:** contrast-aware themes from v0.2 onward; font scaling and screen-reader/TTS consideration for the GUI (currently unscoped — needs its own spike).
- **Config locations:** follow platform conventions (XDG on Linux, equivalents on Windows/Android) for settings, themes, and cache.
- **Encrypted PDFs & scanned/image-only PDFs:** must fail with a clear, explicit "unsupported" message in v0.1–v0.2 rather than silently showing nothing. OCR support is unscoped, to be revisited after v0.2.

## 5. Architecture Summary

```
                    Candi
                      │
                Candi Core
                      │
             ┌────────┴────────┐
             │                 │
            TUI               GUI
             │                 │
         Terminal        Desktop / Android
```

- **candi-core** — document-independent logic: navigation, chapters, bookmarks, search abstraction, reading position, configuration.
- **candi-pdf** — PDF-specific implementation behind a `Document Backend` trait, so no part of the app is hard-coupled to one PDF library.
- **candi-theme** — theme loading, validation, semantic color tokens.
- **candi-tui** — ratatui/crossterm-based terminal frontend.
- **candi-gui** — Slint-based desktop/Android frontend.
- **candi-cli** — argument parsing, document opening, config commands.
- **bindings/python** — PyO3 + maturin wrapper around candi-core.

The core must never know whether it's being driven by the TUI or GUI.

## 6. Default Keybindings (TUI, v0.1)

```
j / ↓   scroll down        h / ←   previous page
k / ↑   scroll up          l / →   next page
g       first page         G       last page
/       search              n / N   next / previous result
q       quit
```

Configurable keybindings are deferred past v0.1.
