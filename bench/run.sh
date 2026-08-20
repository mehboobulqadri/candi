#!/usr/bin/env bash
# Benchmark harness driver. Replicates the spike's methodology
# (spikes/pdf-backend/run.sh): open_ms timed before the file read, best-of-2
# extraction runs per process, process-level RSS (VmRSS baseline -> VmHWM peak
# -> delta), timeout-wrapped runs, nonzero exit on every error path.
set -euo pipefail
trap 'echo "bench/run.sh: FAILED at line $LINENO" >&2; exit 1' ERR

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$ROOT")"
MANIFEST="${BENCH_LOCAL_MANIFEST:-$ROOT/corpus-local.toml}"
GEN_DIR="$ROOT/fixtures/generated"
TIMEOUT="${BENCH_TIMEOUT:-120}"

# Bench targets build to target/release/deps/bench-<hash> (never un-hashed);
# pick the newest built binary.
BENCH="$(ls -t "$REPO"/target/release/deps/bench-* 2>/dev/null | grep -v '\.d$' | head -n 1 || true)"

case "${1:-}" in
  "") FIXTURES_ONLY=0 ;;
  --fixtures-only) FIXTURES_ONLY=1 ;;
  *) echo "usage: $0 [--fixtures-only]" >&2; exit 2 ;;
esac

if [ ! -x "$BENCH" ]; then
  echo "bench/run.sh: ERROR: $BENCH not found — build it first: cargo build --release --benches -p candi-pdf" >&2
  exit 1
fi

# read_manifest <file> <section>: prints "key<TAB>value" lines; any malformed
# line inside the section is a hard fail (never silently drop a book).
read_manifest() {
  awk -v sec="$2" '
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*\[/ {
      insec = ($0 ~ "^[[:space:]]*\\[" sec "\\][[:space:]]*$")
      next
    }
    insec && /^[A-Za-z0-9_-]+[[:space:]]*=[[:space:]]*"[^"]*"[[:space:]]*$/ {
      k = $0; sub(/^[[:space:]]*/, "", k); sub(/[[:space:]]*=.*/, "", k)
      v = $0; sub(/^[^"]*"/, "", v); sub(/"[[:space:]]*$/, "", v)
      print k "\t" v
      next
    }
    insec {
      printf "bench/run.sh: malformed manifest line in [%s]: %s\n", sec, $0 > "/dev/stderr"
      exit 1
    }
  ' "$1"
}

# resolve <pattern>: prints the single resolved path; 0 matches or >1 matches
# is a hard fail (spike's one-corpus-match-per-book assertion). Relative
# patterns resolve against this script's directory (where both manifests live).
resolve() {
  local pattern="$1"
  case "$pattern" in
    /*) ;;
    *) pattern="$ROOT/$pattern" ;;
  esac
  local matches=()
  mapfile -t matches < <(compgen -G "$pattern")
  if [ "${#matches[@]}" -eq 0 ]; then
    echo "bench/run.sh: FILE NOT FOUND: $pattern" >&2
    return 1
  fi
  if [ "${#matches[@]}" -gt 1 ]; then
    echo "bench/run.sh: MULTIPLE MATCHES:" >&2
    printf '  %s\n' "${matches[@]}" >&2
    return 1
  fi
  printf '%s\n' "${matches[0]}"
}

FIXTURES=$(read_manifest "$ROOT/corpus.toml" fixtures)
GEN_ENTRIES=$(read_manifest "$ROOT/corpus.toml" generated)
BOOKS=""
GEN_SOURCE=""

if [ "$FIXTURES_ONLY" = 1 ]; then
  GEN_ENTRIES=""
  echo "bench/run.sh: fixtures-only mode — real books and fixture generation skipped" >&2
elif [ ! -f "$MANIFEST" ]; then
  echo "bench/run.sh: no local corpus manifest at $MANIFEST — fixtures-only run" >&2
  GEN_ENTRIES=""
else
  BOOKS=$(read_manifest "$MANIFEST" books)
  GEN_SOURCE=$(read_manifest "$MANIFEST" generation | awk -F'\t' '$1 == "source" { print $2; exit }')
  if [ -z "$GEN_SOURCE" ]; then
    echo "bench/run.sh: SKIP: no [generation] source in $MANIFEST — generated fixtures (broken.pdf, image-only.pdf) not created" >&2
    GEN_ENTRIES=""
  elif SOURCE=$(resolve "$GEN_SOURCE"); then
    if ! command -v head >/dev/null 2>&1; then
      echo "bench/run.sh: ERROR: head required to generate $GEN_DIR/broken.pdf" >&2
      exit 1
    fi
    if command -v magick >/dev/null 2>&1; then
      IM=magick
    elif command -v convert >/dev/null 2>&1; then
      IM=convert
    else
      echo "bench/run.sh: ERROR: ImageMagick (magick or convert) required to generate $GEN_DIR/image-only.pdf" >&2
      exit 1
    fi
    if ! command -v pdftoppm >/dev/null 2>&1; then
      echo "bench/run.sh: ERROR: pdftoppm (poppler-utils) required to generate $GEN_DIR/image-only.pdf" >&2
      exit 1
    fi
    mkdir -p "$GEN_DIR"
    head -c 1000 "$SOURCE" > "$GEN_DIR/broken.pdf"
    timeout 30 pdftoppm -png -f 1 -l 1 -r 150 "$SOURCE" "$GEN_DIR/imgonly-fixture"
    timeout 30 "$IM" "$GEN_DIR/imgonly-fixture-001.png" "$GEN_DIR/image-only.pdf"
    rm -f "$GEN_DIR/imgonly-fixture-001.png"
    echo "bench/run.sh: generated $GEN_DIR/broken.pdf and $GEN_DIR/image-only.pdf from $SOURCE" >&2
  else
    echo "bench/run.sh: SKIP: generation source not resolvable — generated fixtures (broken.pdf, image-only.pdf) not created" >&2
    GEN_ENTRIES=""
  fi
fi

# One-corpus-match-per-book assertion across both manifests.
DUPES=$(printf '%s\n' "$FIXTURES" "$GEN_ENTRIES" "$BOOKS" | cut -f1 | sort | uniq -d)
if [ -n "$DUPES" ]; then
  echo "bench/run.sh: ERROR: duplicate corpus label(s):" >&2
  printf '  %s\n' $DUPES >&2
  exit 1
fi

echo "note: extraction columns (-) are wired in slice 01/05 when candi-pdf's backend lands" >&2
printf '%-18s %7s %10s %9s %8s %11s %12s %11s\n' \
  "doc" "open_ms" "extract_ms" "chars" "chars/s" "baseline RSS" "process peak" "delta vs baseline"

while IFS=$'\t' read -r label path; do
  [ -n "$label" ] || continue
  if ! resolved=$(resolve "$path"); then
    echo "bench/run.sh: $label: hard fail — no unique file" >&2
    exit 1
  fi
  echo "### $label" >&2
  if ! timeout "$TIMEOUT" "$BENCH" "$label" "$resolved"; then
    echo "bench/run.sh: $label: FAILED or timed out after ${TIMEOUT}s" >&2
    exit 1
  fi
done < <(printf '%s\n%s\n%s\n' "$FIXTURES" "$GEN_ENTRIES" "$BOOKS")