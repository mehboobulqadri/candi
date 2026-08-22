#!/usr/bin/env bash
# Capture one live screenshot of target/release/candi on the Hyprland session.
# usage: shot.sh <pdf> <sidecar.toml|-> <out.png>
#   "-" removes any sidecar first (default-state shot).
set -euo pipefail

pdf=$1 state=$2 out=$3
root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/candi"
sidecar="$pdf.candi.toml"
mkdir -p "$(dirname "$out")"

pkill -f '[t]arget/release/candi' 2>/dev/null || true
sleep 0.3
if [[ $state == - ]]; then
    rm -f "$sidecar"
else
    cp "$state" "$sidecar"
fi

"$bin" "$pdf" > /tmp/candi-shots/app.log 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true' EXIT
sleep 3

win=$(hyprctl -j clients | jq -c 'first(.[] | select(.mapped and (.class|ascii_downcase|contains("candi")) or (.title|test("Candi"))))')
if [[ -z $win ]]; then
    echo "no candi window found" >&2
    exit 1
fi
x=$(jq '.at[0]' <<<"$win"); y=$(jq '.at[1]' <<<"$win")
w=$(jq '.size[0]' <<<"$win"); h=$(jq '.size[1]' <<<"$win")
echo "window ${x},${y} ${w}x${h} (class $(jq -r .class <<<"$win"), title $(jq -r .title <<<"$win"))"

mkdir -p /tmp/candi-shots
grim -g "${x},${y} ${w}x${h}" "$out"
bytes=$(stat -c%s "$out")
if [[ $bytes -lt 10000 ]]; then
    echo "suspiciously small screenshot ($bytes bytes): $out" >&2
    exit 1
fi
echo "captured $out ($bytes bytes)"

kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
trap - EXIT
