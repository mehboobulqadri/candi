# Candi — a minimal, fast, cross-platform document reader

A document reader with a TUI and a GUI.

> One reader. One core. Two interfaces.

## Status

Feasibility complete; all decisions made; planning done; repo at base (docs + spike
results, no code yet).

- Spike 1 (PDF backend) resolved: MuPDF + PDFium ship in v1 as a dual, runtime-switchable
  backend behind one trait; Poppler rejected. License: AGPL-3.0, with a permissive-build
  escape hatch (pdfium-only feature build).
- [Architecture](docs/architecture.md) is written and guides all further work.
- [Implementation](docs/implementation/README.md) is complete as a phase/slice plan —
  one phase per version, each phase delivered as reviewable slice commits; see
  [progress](docs/implementation/progress.md) for where we are.
- Next: implementation phase 00 — workspace bootstrap (see [workflow](docs/workflow.md)).

## Project documents

- [Project](docs/project.md)
- [Requirements](docs/REQUIREMENTS.md)
- [Spikes and options](docs/spikes.md)
- [Workflow](docs/workflow.md)
- [Architecture](docs/architecture.md)
- [Implementation](docs/implementation/README.md) — phase/slice build plan and working method

Public documentation sources live under `docs/`. The site is published with
GitHub Pages — see [docs/publishing.md](docs/publishing.md) for how.
