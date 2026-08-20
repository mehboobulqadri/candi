# Security Policy

Candi is a document reader. PDFs are untrusted, attacker-controlled input — the
parser and extraction code of the two C engines (MuPDF, PDFium) is the attack
surface. See [docs/architecture.md §Security](docs/architecture.md) for the threat
model and mitigations.

## Supported versions

None yet — Candi has no releases. The first supported version is v0.1.

## Reporting a vulnerability

**Do not open a public issue for security problems.** Report privately:

- Preferred: a [private security advisory] on this repository (GitHub → Security →
  Report a vulnerability).
- Fallback: email `security@candi.dev` (placeholder — TODO: set the real address
  before v0.1).

Please include: Candi version, backend in use (`--backend mupdf|pdfium`), the
document that triggered the issue (or a minimized reproducer), steps to reproduce,
and relevant logs. Logs live locally at `~/.local/state/candi/logs/` — never paste
document contents or personal data into a report.

## Scope

- In scope: the Candi application and PDF parsing/extraction paths.
- Out of scope: vulnerabilities in third-party engines that are not reachable from
  Candi's own code paths.

## Expectations

- **No telemetry.** Candi sends nothing off the machine; diagnostics stay in local
  logs.
- The project is AGPL-3.0 — a private disclosure may be coordinated with the
  public fix release.