---
title: Roadmap & Versioning
nav_order: 6
---

# Candi — Roadmap & Versioning Rules

## Versioning rules

- **Major updates** are versions: **v0.x**. Each carries a batch of features.
- **Minor and fix updates** after a release are patch versions: **v0.x.y**.
- **Features merge to `dev`.** `dev` merges to `main` for a major release (a new version).
- **New feature branches start after a new version is cut**, never mid-cycle.

## The timeline

### v0.1 — shipped base + UI fixes *(current)*

The working reader.

- Graphical reader (`candi`): rendered PDF pages,
  continuous / single / dual flows, fit-width & fit-page, anchored zoom, search with
  in-page highlights, bookmarks, table of contents.
- Dual PDF backends behind one trait (MuPDF default; PDFium permissive build), per-PDF
  resume sidecars, recents, 5 built-in themes + theme editor + custom YAML themes,
  editable keybindings.

### v0.2 — more formats, locked PDFs, polish

- EPUB and other document formats beyond PDF.
- Password-locked PDF support with a proper unlock flow.
- Optimizations and cleanup across the core and the frontend.

### v0.3 — multi-OS

- Windows support alongside Linux on other distributions.
- Deployment story per platform (installers/bundles).

### v0.4 — documentation

- GitHub-hosted documentation and a cleanup pass over the repo docs.

### v0.5 — packaging + CI

- AUR package and Debian packaging, plus CI to keep them fresh.
  *Note:* the user wants the AUR attempt pulled early — expect first packaging
  experiments inside the v0.1.x/v0.2 timeframe ahead of this milestone.

### v0.6 — Android

- Android port (beta) with UI optimization for touch, and mobile-friendly CI.
