#!/usr/bin/env bash
# Capture live GUI screenshots of target/release/candi on the Hyprland session.
#
# usage:
#   shot.sh <pdf> <sidecar.toml|-> <out.png>   single capture ("-" = no sidecar)
#   shot.sh all <out-dir>                      slice-I verification matrix
#   shot.sh empty <out.png>                    no-args launch probe (rfd may block)
#
# Each matrix entry is a fresh launch. Sidecars are schema-v2 sessions written
# next to the PDF copy; page numbers are 0-based. Window geometry is re-read
# via `hyprctl -j clients` before every grim call; only candi processes spawned
# here are ever killed.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/candi"
work=${CANDI_SHOT_WORK:-/tmp/candi-shots}
mkdir -p "$work"
pid=0
trap '[[ $pid -gt 0 ]] && kill "$pid" 2>/dev/null || true' EXIT

wait_for_window() {
    for _ in $(seq 1 50); do
        win=$(hyprctl -j clients | jq -c 'first(.[] | select(.mapped and (.class|ascii_downcase|contains("candi")) or (.title|test("Candi"))))' || true)
        [[ -n $win && $win != null ]] && return 0
        sleep 0.2
    done
    echo "no candi window found" >&2
    return 1
}

capture_window() {
    local out=$1
    # Re-read geometry immediately before every shot.
    win=$(hyprctl -j clients | jq -c 'first(.[] | select(.mapped and (.class|ascii_downcase|contains("candi")) or (.title|test("Candi"))))')
    if [[ -z $win || $win == null ]]; then
        echo "no candi window found" >&2
        return 1
    fi
    local x y w h bytes
    x=$(jq '.at[0]' <<<"$win"); y=$(jq '.at[1]' <<<"$win")
    w=$(jq '.size[0]' <<<"$win"); h=$(jq '.size[1]' <<<"$win")
    echo "window ${x},${y} ${w}x${h} (class $(jq -r .class <<<"$win"), title $(jq -r .title <<<"$win"))"
    mkdir -p "$(dirname "$out")"
    grim -g "${x},${y} ${w}x${h}" "$out"
    bytes=$(stat -c%s "$out")
    if (( bytes < 10000 )); then
        echo "suspiciously small screenshot ($bytes bytes): $out" >&2
        return 1
    fi
    echo "captured $out ($bytes bytes)"
}

launch_and_capture() {
    local pdf=$1 sidecar=$2 out=$3
    pkill -f '[t]arget/release/candi' 2>/dev/null || true
    sleep 0.3
    if [[ $sidecar == - ]]; then
        rm -f "$pdf.candi.toml"
    else
        cp "$sidecar" "$pdf.candi.toml"
    fi

    "$bin" "$pdf" > "$work/app.log" 2>&1 &
    pid=$!
    sleep 3

    capture_window "$out"

    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    pid=0
}

write_session() {
    # write_session <out.toml> <page-0-based> <zoom> <theme> [bookmarks...]
    local out=$1 page=$2 zoom=$3 theme=$4
    shift 4
    {
        printf 'schema_version = 2\nupdated_at = "2026-08-21T00:00:00Z"\n\n[reading]\n'
        printf 'page = %d\nscroll_frac = 0.0\nzoom = %s\ntheme = "%s"\n' "$page" "$zoom" "$theme"
        for b in "$@"; do
            printf '\n[[bookmarks]]\npage = %d\ncreated_at = "2026-08-21T00:00:00Z"\n' "$b"
        done
    } > "$out"
}

matrix() {
    local dir=$1 pdf="$work/demo-book.pdf"
    if [[ ! -f $pdf ]]; then
        echo "demo PDF missing: $pdf (copy a multi-page fixture there first)" >&2
        return 1
    fi

    write_session "$work/states/sepia.toml"       0 '"fit-width"' "Sepia"
    write_session "$work/states/warm-dark.toml"   0 '"fit-width"' "Warm Dark"
    write_session "$work/states/dark.toml"        0 '"fit-width"' "Dark"
    write_session "$work/states/true-dark.toml"   0 '"fit-width"' "True Dark"
    write_session "$work/states/zoom150.toml"     0 150           "Light"
    write_session "$work/states/page6.toml"       6 '"fit-width"' "Light"
    write_session "$work/states/bookmarked.toml"  1 '"fit-width"' "Light" 1 8

    launch_and_capture "$pdf" -                                "$dir/01-light-fit-width.png"
    launch_and_capture "$pdf" "$work/states/sepia.toml"        "$dir/02-sepia.png"
    launch_and_capture "$pdf" "$work/states/warm-dark.toml"    "$dir/03-warm-dark.png"
    launch_and_capture "$pdf" "$work/states/dark.toml"         "$dir/04-dark.png"
    launch_and_capture "$pdf" "$work/states/true-dark.toml"    "$dir/05-true-dark.png"
    launch_and_capture "$pdf" "$work/states/zoom150.toml"      "$dir/06-zoom-150.png"
    launch_and_capture "$pdf" "$work/states/page6.toml"        "$dir/07-page6-scrolled.png"
    launch_and_capture "$pdf" "$work/states/bookmarked.toml"   "$dir/08-bookmarked.png"
    launch_and_capture "$work/nonexistent.pdf" -               "$dir/09-error-state.png"

    # Tagline-hiding probe: float + resize the candi window via the Lua
    # dispatcher API (Hyprland 0.56 routes `hyprctl dispatch` through Lua and
    # legacy dispatcher names fail). Guarded: every dispatch only runs while
    # our own candi window holds focus.
    pkill -f '[t]arget/release/candi' 2>/dev/null || true
    sleep 0.3
    rm -f "$pdf.candi.toml"
    "$bin" "$pdf" > "$work/app.log" 2>&1 &
    pid=$!
    sleep 3
    active=$(hyprctl -j activewindow | jq -r '.class // ""')
    if [[ $active != candi ]]; then
        echo "candi window not focused; skipping narrow resize (would touch user window)" >&2
        kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; pid=0
        return 1
    fi
    hyprctl dispatch 'hl.dsp.window.float({ action = "toggle" })' > /dev/null
    sleep 0.7
    read w h <<<"$(hyprctl -j clients | jq -r '.[] | select(.class=="candi") | "\(.size[0]) \(.size[1])"')"
    hyprctl dispatch "hl.dsp.window.resize({ x = $((700 - w)), y = $((800 - h)), relative = true })" > /dev/null
    sleep 0.7
    capture_window "$dir/10-light-narrow-tagline.png"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    pid=0
}

empty_probe() {
    local out=$1
    pkill -f '[t]arget/release/candi' 2>/dev/null || true
    sleep 0.3
    "$bin" > "$work/empty.log" 2>&1 &
    pid=$!
    sleep 4
    if wait_for_window; then
        sleep 1
        capture_window "$out"
    else
        echo "no window mapped after no-args launch: rfd file dialog blocks headlessly" >&2
        kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; pid=0
        return 1
    fi
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    pid=0
}

case ${1:-} in
    all)   shift; matrix "${1:?usage: shot.sh all <out-dir>}"; pkill -f '[t]arget/release/candi' 2>/dev/null || true ;;
    empty) shift; empty_probe "${1:?usage: shot.sh empty <out.png>}"; pkill -f '[t]arget/release/candi' 2>/dev/null || true ;;
    *)     [[ $# -eq 3 ]] || { sed -n '2,12p' "$0"; exit 2; }; launch_and_capture "$@" ;;
esac
