#!/bin/sh
# Installs the candi release artifacts under $PREFIX (default ~/.local).
set -eu

PREFIX="${PREFIX:-$HOME/.local}"
cd "$(dirname "$0")"

install -Dm0755 candi "$PREFIX/bin/candi"

for size in 16 24 32 48 64 128 256 512; do
  install -Dm0644 "icons/candi-${size}.png" \
    "$PREFIX/share/icons/hicolor/${size}x${size}/apps/candi.png"
done

install -Dm0644 candi.desktop "$PREFIX/share/applications/candi.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$PREFIX/share/applications"
fi

echo "Installed. If $PREFIX/bin is not on your PATH, add it."
echo "Run: candi book.pdf"
