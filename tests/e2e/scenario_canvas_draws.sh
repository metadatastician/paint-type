#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
#
# scenario_canvas_draws.sh -- Tier C: prove the product actually draws.
#
# This is the first test in this repository that runs the `paint-type` binary.
# Its sibling scenario_host_headless.sh is named as though it covers the host
# but drives host_core only: host_core has no gossamer-rs dependency, so it
# never links libgossamer, never initialises GTK, never opens a display and
# never executes this binary. The two are complementary, not redundant -- see
# "Isolation" at the bottom.
#
# What a pass here proves, end to end:
#   GTK initialised -> WebKit loaded the page -> JS ran -> the Gossamer bridge
#   round-tripped -> paint_core rasterised a stroke -> the PNG encoder wrote it.
#
# Requires a build first (scripts/build-host.sh) plus xvfb-run.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="$ROOT/src/host/target/release/paint-type"
LIBDIR="$ROOT/third_party/gossamer/src/interface/ffi/zig-out/lib"
PROBE="$ROOT/tests/fixtures/canvas-probe.html"
LOG="${PT_PROBE_LOG:-/tmp/pt-probe.log}"

BLANK=/tmp/pt-blank.png
CANVAS=/tmp/pt-canvas.png
DONE=/tmp/pt-done.png

die() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

[ -x "$BIN" ]   || die "no binary at $BIN -- run scripts/build-host.sh first"
[ -f "$PROBE" ] || die "probe fixture missing at $PROBE"

# Mandatory. Without this a re-run passes on last run's files even if this
# build never wrote a byte -- the classic stale-artifact false green.
rm -f "$BLANK" "$CANVAS" "$DONE" "$LOG"

XVFB_SCREEN="-screen 0 1280x1024x24"

# Display strategy. CI installs xvfb and has no real display; a developer box
# (WSLg, or any desktop session) has DISPLAY and usually no xvfb. Supporting
# both is what lets this script be the LOCAL reproduction of the CI failure
# rather than a thing that only ever runs on a runner.
if command -v xvfb-run >/dev/null 2>&1; then
    display_cmd=(xvfb-run -a -s "$XVFB_SCREEN")
    printf '==> display: xvfb-run\n'
elif [ -n "${DISPLAY:-}" ]; then
    display_cmd=()
    printf '==> display: existing DISPLAY=%s\n' "$DISPLAY"
else
    die "no xvfb-run and no DISPLAY -- cannot start a GUI process"
fi
# Software rendering: the runner has no GPU, and WebKit's compositor and dmabuf
# renderer both fail on a bare Xvfb in ways that look like application crashes.
webkit_env=(
    WEBKIT_DISABLE_COMPOSITING_MODE=1
    WEBKIT_DISABLE_DMABUF_RENDERER=1
    LIBGL_ALWAYS_SOFTWARE=1
    GDK_BACKEND=x11
)

printf '==> launching paint-type with PT_UI_FILE=%s\n' "$PROBE"
# build.rs emits a bare -lgossamer with no rpath, and the packaging rpath points
# at target/lib, which does not exist in-tree. So the library path is required
# here, and that requirement is exactly what the negative control below tests.
LD_LIBRARY_PATH="$LIBDIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
    PT_UI_FILE="$PROBE" \
    timeout 90 "${display_cmd[@]}" env "${webkit_env[@]}" "$BIN" \
    >"$LOG" 2>&1 &
app_pid=$!

# Poll for the marker instead of sleeping a fixed interval. A fixed sleep is
# either too short (flaky) or too slow, and it cannot tell "still working" from
# "hung": the marker is written last, so its presence means the whole chain ran.
deadline=$((SECONDS + 75))
while [ ! -s "$DONE" ] && [ "$SECONDS" -lt "$deadline" ]; do
    kill -0 "$app_pid" 2>/dev/null || break   # died early; stop waiting
    sleep 1
done

kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true

if [ ! -s "$DONE" ]; then
    printf -- '--- %s ---\n' "$LOG"
    cat "$LOG" 2>/dev/null || true
    die "probe never wrote its completion marker; the chain did not finish"
fi

[ -s "$BLANK" ]  || die "no blank baseline at $BLANK"
[ -s "$CANVAS" ] || die "no painted canvas at $CANVAS"

# Dimensions must match, or `cmp` below would report "differ" simply because the
# two files are different sizes -- passing the gate while proving nothing.
blank_geom=$(file -b "$BLANK"  | sed 's/.*, \([0-9]* x [0-9]*\),.*/\1/')
canvas_geom=$(file -b "$CANVAS" | sed 's/.*, \([0-9]* x [0-9]*\),.*/\1/')
[ "$blank_geom" = "$canvas_geom" ] \
    || die "geometry mismatch: blank=$blank_geom canvas=$canvas_geom"

# THE assertion. `file ... 'PNG image data'` is vacuous here: SavePng emits a
# structurally valid PNG whether or not a single pixel was ever touched, so a
# no-op paint path would sail through it. Difference from an untouched document
# of identical dimensions is what actually says "it drew".
if cmp -s "$BLANK" "$CANVAS"; then
    die "canvas is byte-identical to the untouched baseline -- nothing was painted"
fi

printf 'PASS: canvas differs from baseline (%s)\n' "$blank_geom"
printf '      blank  %s bytes\n' "$(wc -c <"$BLANK")"
printf '      canvas %s bytes\n' "$(wc -c <"$CANVAS")"

# ---------------------------------------------------------------------------
# Negative control. Without it the assertion above is untrustworthy: a launch
# test that cannot fail proves nothing about what makes the launch work.
#
# Identical to the positive run in every respect except LD_LIBRARY_PATH -- same
# binary, same xvfb, same WebKit env -- so a difference in outcome isolates to
# that one variable. Running it under xvfb matters: outside a display the binary
# dies of WebviewCreateFailed regardless, which would confound the two causes.
# ---------------------------------------------------------------------------
printf '==> negative control: same launch, no LD_LIBRARY_PATH\n'
set +e
env -u LD_LIBRARY_PATH \
    timeout 15 "${display_cmd[@]}" env "${webkit_env[@]}" "$BIN" \
    >/tmp/pt-control.log 2>&1
control_rc=$?
set -e

# 124 is timeout's "still running", i.e. it launched successfully. If the binary
# survives without the library path, then either libgossamer is being resolved
# some other way or the rpath resolves in-tree -- and in both cases the positive
# run above no longer demonstrates what it claims.
if [ "$control_rc" -eq 124 ]; then
    die "control survived without LD_LIBRARY_PATH (rc=124); the positive result is vacuous"
fi
printf 'PASS: control died as expected (rc=%s)\n' "$control_rc"

# ---------------------------------------------------------------------------
# Isolation. Run alongside scenario_host_headless.sh, which drives the same
# raster core with no webview. If that one writes its PNG and this one does not,
# the defect is in the webview or the bridge and provably not in paint_core.
# ---------------------------------------------------------------------------
printf 'PASS: Tier C -- paint-type launched, rendered and saved\n'
