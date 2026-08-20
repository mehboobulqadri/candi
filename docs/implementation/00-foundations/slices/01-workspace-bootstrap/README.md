# Slice 00/01 — workspace-bootstrap

## Goal

A Cargo workspace with the five v0.1 crates as empty skeletons, building cleanly
with the drill applied — the base every later slice lands on.

## Prep / thinking

- **Workspace layout:** `[workspace] resolver = "2"`, `members = ["crates/*"]`;
  release profile `opt-level = 3`, `lto = "thin"`.
- **Crate set** exactly per project.md §5: `crates/candi-core`, `crates/candi-pdf`,
  `crates/candi-theme` (lib), `crates/candi-tui` (lib + bin), `crates/candi-cli`
  (bin). No `candi-gui`, no `bindings/python` — created when their phases start.
- **Dependency direction** is established now even with zero deps:
  cli → tui/core/pdf; tui/core → candi-pdf (trait only); core never depends on
  frontends (architecture.md workspace diagram, standing rule 4).
- **Edition + toolchain:** pin current stable via `rust-toolchain.toml` (the spike
  used 1.97.1); the pin is deliberate — document the choice in the PR.
- **LICENSE:** AGPL-3.0 full text at repo root, one license for the workspace; the
  permissive escape hatch is a cargo feature, not a separate license
  (architecture.md §License implications).
- `Cargo.lock` is committed — application workspace (`.gitignore` already notes
  this).
- Empty `lib.rs`/`main.rs` files are fine; the workspace must build with **zero
  warnings** (a warning here becomes a CI failure in slice 00/02).

## Files

- `Cargo.toml` (workspace)
- `rust-toolchain.toml`
- `LICENSE` (AGPL-3.0)
- `crates/candi-core/{Cargo.toml, src/lib.rs}`
- `crates/candi-pdf/{Cargo.toml, src/lib.rs}`
- `crates/candi-theme/{Cargo.toml, src/lib.rs}`
- `crates/candi-tui/{Cargo.toml, src/lib.rs, src/main.rs}`
- `crates/candi-cli/{Cargo.toml, src/main.rs}`

## Implementation tasks

1. Write the workspace `Cargo.toml` (resolver 2, members `crates/*`, release
   profile).
2. Create the five crates per project.md §5 (tui = lib + bin; cli = bin).
3. Add `rust-toolchain.toml` pinning current stable.
4. Add AGPL-3.0 `LICENSE` at repo root.
5. `cargo build` + `cargo test` — workspace green, zero warnings.
6. Run the drill.

## Verification

- Drill stage 2: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
  deslop pass (nothing to deslop — keep it that way).
- `cargo build --release` succeeds.
- Drill stage 4: independent line-by-line review of the whole diff.
- Tests: none yet — the workspace itself is the deliverable; CI proves it in the
  next slice.

## Commit message

```
build: bootstrap cargo workspace with empty crates
```

## PR notes

- Merge target: `dev`.
- Reviewer: crate set matches project.md §5 (no candi-gui, no bindings); members
  pattern `crates/*`; release profile; LICENSE is AGPL-3.0; no dead config, zero
  warnings.

## Risks

- Edition/toolchain choice is sticky — pin current stable deliberately.
- Zero-warning requirement — enforce now, it compounds later.