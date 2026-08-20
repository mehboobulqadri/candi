---
title: Publishing
nav_order: 7
---

# Publishing this as GitHub Pages

1. The `docs/` folder sits at the root of the repo.
2. Commit and push to `main`.
3. In GitHub: repo → **Settings → Pages**.
4. Under "Build and deployment", set **Source** to "Deploy from a branch".
5. Set **Branch** to `main` and folder to `/docs`, then Save.
6. GitHub builds the site with Jekyll automatically (the `_config.yml` here already sets the theme) — live in a minute or two at `https://<your-username>.github.io/<repo-name>/`.

No further setup needed — no Gemfile, no local Jekyll install required for GitHub's default build.

## Docs tree layout

```text
docs/
├── index.md
├── project.md
├── REQUIREMENTS.md
├── spikes.md
├── workflow.md
├── architecture.md
├── implementation/
│   ├── README.md        # plan index + working method (site page at implementation/)
│   ├── progress.md
│   ├── 00-foundations/  # phase dirs: README.md + slices/<NN-name>/README.md
│   ├── 01-v0.1/
│   ├── 02-v0.2/
│   ├── 03-v0.3/
│   ├── 04-v0.4/
│   ├── 05-v0.5/
│   └── 06-v0.9/
├── publishing.md
└── _config.yml
```

Notes:

- The implementation plan is a **directory**, not a file. GitHub Pages serves
  `implementation/README.md` as the page at `implementation/` — so site links use
  `implementation/` (no `.html`), while raw-GitHub links use
  `implementation/README.md`. The Jekyll deploy (`docs/.github/workflows/pages.yml`,
  `cp -r docs/* _site/`) copies the directory recursively — no workflow change
  needed.
- Nested phase/slice READMEs render on GitHub as plain documents; only the
  top-level README carries Jekyll front matter (title/nav_order).
- `spikes/results/` (spike result docs) lives **outside** `docs/` — it is
  committed to the repo but not published by Jekyll. The old `spike-1-results.md`
  entry in this tree was wrong (the file lives at `spikes/results/`).

## Deployment flow

```text
Developer edits docs
        ↓
git push
        ↓
GitHub Actions (docs/.github/workflows/pages.yml)
        ↓
build static site
        ↓
upload Pages artifact
        ↓
GitHub Pages
```

A Pages workflow should use the repository's chosen static-site generator.
