#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -f creds.yml ]; then
  echo "creds.yml not found — copy the values from a project owner; it is gitignored" >&2
  exit 1
fi

keys="name email github project license"
sed_args=()
for key in $keys; do
  value="$(sed -n "s/^${key}: *//p" creds.yml)"
  if [ -z "$value" ]; then
    echo "creds.yml: missing value for '$key'" >&2
    exit 1
  fi
  escaped="$(printf '%s' "$value" | sed 's/[&/\]/\\&/g')"
  sed_args+=(-e "s/{{${key}}}/${escaped}/g")
done

files="README.md SECURITY.md LICENSE.md $(find docs -name '*.md' | sort)"

count=0
for f in $files; do
  if ! grep -qE '\{\{(name|email|github|project|license)\}\}' "$f"; then
    continue
  fi
  count=$((count + 1))
  before="$(grep -oE '\{\{(name|email|github|project|license)\}\}' "$f" | wc -l || true)"
  sed -i "${sed_args[@]}" "$f"
  after="$(grep -oE '\{\{(name|email|github|project|license)\}\}' "$f" | wc -l || true)"
  echo "$f: $((before - after)) placeholder(s) replaced"
  if [ "$after" -gt 0 ]; then
    echo "$f: FAILED — leftover placeholders: $(grep -oE '\{\{(name|email|github|project|license)\}\}' "$f" | sort -u | tr '\n' ' ')" >&2
    exit 1
  fi
done
echo "Processed $count files"
