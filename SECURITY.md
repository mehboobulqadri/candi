# Security Policy

Candi is a document reader. PDFs are untrusted, attacker-controlled input — the
parser and extraction code of the two C engines (MuPDF, PDFium) is the attack
surface. See [docs/architecture.md §Security](docs/architecture.md) for the threat
model and mitigations.

## Supported versions

None yet — Candi has no releases. The first supported version is v0.1.

## Reporting a vulnerability

**Do not open a public issue for security problems.** Report privately:

- Preferred: a [private security advisory](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability) on this repository (GitHub → Security →
  Report a vulnerability).
- Fallback: email `mehboobulqadri@gmail.com`.

Please include: Candi version, backend in use (`--backend mupdf|pdfium`), the
document that triggered the issue (or a minimized reproducer), steps to reproduce,
and relevant logs. v0.1 ships diagnostics to stderr behind `--verbose`; file logs at `~/.local/state/candi/logs/` arrive with the tracing-based logging planned in the implementation docs — never paste
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
