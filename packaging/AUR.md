# Candi on the AUR

`packaging/PKGBUILD` builds a **source package** (`candi`, no `-bin`/`-git`
suffix) from the GitHub tag tarball `v0.1.0`. Tag-day work is at the bottom.

## What the package contains

Two binaries installed to `/usr/bin`: `candi` (GUI) and `candi-tui`. The
package installs the AGPL-3.0 license text, a freedesktop `.desktop` entry
(Open With / menu integration for `application/pdf`) and a 128px hicolor icon.
The build uses the crate defaults (MuPDF statically linked into the binaries
by `mupdf-sys`, which compiles a bundled copy of MuPDF's C sources with clang
at build time), so there is no dynamic dependency on any system MuPDF library.
The PDFium backend is compiled in but only ever used if a standalone
`libpdfium.so` is found next to the executable or via `PDFIUM_LIB`; without it
the app runs MuPDF-only, which is exactly what the distro build intends.

## One-time submission (user executes)

1. Create an account on <https://aur.archlinux.org>.
2. Dedicated SSH key:
   ```
   ssh-keygen -f ~/.ssh/aur -C "aur-candi"
   # upload ~/.ssh/aur.pub under your AUR profile: My Account → SSH Public Key
   cat >> ~/.ssh/config <<'EOF'
   Host aur.archlinux.org
     IdentityFile ~/.ssh/aur
     User aur
   EOF
   ssh-keyscan aur.archlinux.org >> ~/.ssh/known_hosts
   ```
3. Verify no name collision first:
   <https://aur.archlinux.org/packages/candi> must NOT exist yet.
4. Clone and seed:
   ```
   git clone ssh://aur@aur.archlinux.org/candi.git
   cd candi            # warning "empty repository" is normal; if not empty → collision, STOP
   cp ~/Projects/candi/packaging/{PKGBUILD,.SRCINFO} .
   updpkgsums          # fetches the v0.1.0 tarball, writes real b2sums over SKIP
   makepkg --printsrcinfo > .SRCINFO   # ALWAYS regenerate after PKGBUILD edits, never hand-edit
   git add PKGBUILD .SRCINFO && git commit -m "candi 0.1.0"
   git push origin master      # master only — pushing elsewhere is a rejection reason
   ```

## Tag-day update flow (every release)

1. Cut tag `v<pkgver>` on GitHub.
2. In the repo's `packaging/`: bump `pkgver=…`, reset `pkgrel=1`.
3. `updpkgsums` (against packaging/PKGBUILD once the tag exists), then
   `makepkg --printsrcinfo > .SRCINFO`.
4. Commit both files, mirror into the AUR clone, commit, push to `master`.

## Rejection reasons avoided by this layout

- missing `# Maintainer:` first line → present
- stale/hand-edited `.SRCINFO` → always regenerated
- non-`master` branch pushes → guide pins master
- `-bin` suffix on a source build → pkgname is plain `candi`
- `replaces=` misuse → not used

## Local validation performed for this version

Full `makepkg -f` cycle (prepare/build/check/package) passed against a local
snapshot of this tree; packaged tree verified to contain both binaries,
license, desktop file and icon; binaries execute. Note: this machine has no
`namcap` installed, so namcap linting was skipped.
