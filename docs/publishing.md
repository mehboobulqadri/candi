---
title: Publishing
nav_order: 7
---

# Publishing this as GitHub Pages

**Status: deferred.** GitHub Pages is not set up yet — it will be once the
project is stable and the docs are in final shape. This page records the plan.

When set up, the site is a plain Jekyll build of `docs/` — no Gemfile or local
Jekyll install needed, GitHub's built-in build serves `docs/` directly from
`main` — telling about the **project** only. Content is exposed
**progressively** as the project matures, re-evaluated at each phase merge:

- **Stage 1** (v0.1 stable): index + project
- **Stage 2** (v0.2+): requirements + architecture
- **Later stages**: decided at the time; only project-facing content

NEVER published: `implementation/`, `spikes/`, `knowledge/` — project-internal
working docs, excluded via `_config.yml`; spike results live outside `docs/`
(repo-root `spikes/results/`) and are never published either.

## The plan (for when Pages is enabled)

1. The `docs/` folder sits at the root of the repo; commit and push to `main`.
2. In GitHub: repo → **Settings → Pages**.
3. Under "Build and deployment", set **Source** to "Deploy from a branch".
4. Set **Branch** to `main` and folder to `/docs`, then Save.
5. GitHub builds the site with Jekyll automatically (the `_config.yml` here already sets the theme) — live in a minute or two at `https://<your-username>.github.io/<repo-name>/`.

## Docs tree layout

```text
docs/
├── index.md
├── project.md
├── REQUIREMENTS.md
├── spikes.md
├── workflow.md
├── architecture.md
├── implementation/   # project-internal — excluded from the site
├── knowledge/        # project-internal — excluded from the site
├── publishing.md
└── _config.yml
```
