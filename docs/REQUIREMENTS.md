---
title: Requirements
nav_order: 2
---

# Candi — Requirements Specification

## v0.1 functional requirements

### FR-001 Open PDF
Accept a PDF path from the command line, e.g. `candi book.pdf`, and report useful filesystem/open errors.

### FR-002 Validate input
Determine whether the input is a readable PDF. Malformed or unsupported documents must produce recoverable errors.

### FR-003 Detect text
Determine whether an accessible text layer exists. Image-only PDFs must produce an explicit OCR-not-supported message in v0.1.

### FR-004 Extract text
Extract text while preserving enough positional information to support future reading-order reconstruction.

### FR-005 Display text
Render extracted text in a terminal-readable layout. Pixel-perfect PDF reproduction is not required for v0.1.

### FR-006 Scroll
Scroll through the document/page.

### FR-007 Page navigation
Move between pages.

### FR-008 Search
Search extracted text and navigate between results.

### FR-009 Reading position
Remember the last reading position outside the original PDF.

### FR-010 Graceful errors
PDF, extraction, filesystem, and unsupported-feature failures must not cause avoidable crashes.

## Non-functional requirements

- Fast-feeling startup and navigation.
- Responsive TUI during expensive operations.
- Lazy loading rather than loading every page into memory.
- OS-independent core.
- No Python runtime required by the native app.
- Core operations testable without launching a UI.
- Benchmarks must establish actual performance before optimization claims.

## PDF edge cases

Explicitly test normal PDFs, multi-column papers, scanned PDFs, image-heavy PDFs, encrypted/password-protected PDFs, malformed PDFs, and unsupported PDF features.

## Future requirements

TOC, bookmarks, custom chapters, themes, dark mode, warm mode, GUI, drag/drop, graphical PDF rendering, images, diagrams, Android, Python API, terminal graphics, OCR, annotations, and other document formats.

## Theme requirements

Themes must use semantic tokens and be independent of UI implementation. Invalid themes must fall back safely and report useful errors.

## State requirements

Document state must be versioned, separate from the PDF, safely writable, tolerant of missing state, and eventually support migration and concurrent-access handling.

## Security requirements

PDFs are untrusted input. Investigate malformed PDFs, parser vulnerabilities, resource exhaustion, embedded JavaScript, external links, file access, backend security, and sandboxing. Candi must not execute arbitrary PDF content.

## Distribution requirements

Long-term targets: PyPI, AUR, CachyOS, Debian/Ubuntu, Windows, Android. Native Candi must remain independent of Python.

## v0.1 acceptance criteria

A candidate release must open representative real-world PDFs, display text, navigate, scroll, search, persist position, clearly report image-only PDFs, handle invalid input without crashing, run on Linux and Windows, produce benchmark results, and use a PDF backend whose licensing is compatible with the distribution strategy.
