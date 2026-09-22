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
PROBE_TEMPLATE="$ROOT/tests/fixtures/canvas-probe.html"

# Every artefact this run produces lives in ONE directory that belongs to this
# run alone. Shared /tmp paths let two concurrent runs read or delete each
# other's PNGs, and a run that consumed another run's canvas would report a
# result it never earned -- a false green that no assertion below could catch.
#
# PT_RUN_DIR lets a harness pin the location so it can collect artefacts after
# a failure (host.yml does this). We clean up ONLY a directory we created
# ourselves: deleting a caller's directory would destroy the diagnostics the
# caller asked for.
if [ -n "${PT_RUN_DIR:-}" ]; then
    RUNDIR="$PT_RUN_DIR"
    mkdir -p "$RUNDIR"
    owns_rundir=0
else
    RUNDIR="$(mktemp -d "${TMPDIR:-/tmp}/pt-canvas-proof.XXXXXX")"
    owns_rundir=1
fi
# ⚠ An EXIT trap whose LAST command returns nonzero OVERWRITES the script’s exit
# status on an otherwise-successful run. With PT_RUN_DIR set (always, in CI)
# owns_rundir is 0, so a bare `[ … ] && rm -rf` short-circuits to 1 and turns a
# PASSING canvas proof red. The explicit `if`/`return 0` keeps a real failure.
cleanup() { if [ "$owns_rundir" -eq 1 ]; then rm -rf "$RUNDIR"; fi; return 0; }
trap cleanup EXIT

LOG="$RUNDIR/pt-probe.log"
CONTROL_LOG="$RUNDIR/pt-control.log"
PROBE="$RUNDIR/canvas-probe.html"
BLANK="$RUNDIR/pt-blank.png"
CANVAS="$RUNDIR/pt-canvas.png"
DONE="$RUNDIR/pt-done.png"

die() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

[ -x "$BIN" ]            || die "no binary at $BIN -- run scripts/build-host.sh first"
[ -f "$PROBE_TEMPLATE" ] || die "probe fixture missing at $PROBE_TEMPLATE"

printf '==> run directory: %s\n' "$RUNDIR"

# The host loads the page with `load_html` -- an HTML STRING, not a file URL --
# so the page has no `location` to derive its own output directory from. Hence
# substitution. `|` as the sed delimiter because RUNDIR contains slashes.
sed "s|__PT_OUT_DIR__|$RUNDIR|g" "$PROBE_TEMPLATE" > "$PROBE"

# Control on the substitution itself. A sed that silently did nothing would
# send the browser a literal `__PT_OUT_DIR__/pt-done.png`, the marker would
# never appear at the path this script polls, and the failure would read as
# "the product does not draw" -- the wrong diagnosis entirely.
if command grep -Fq '__PT_OUT_DIR__' "$PROBE"; then
    die "output-directory substitution failed; $PROBE still holds the placeholder"
fi

# Mandatory even in a fresh directory: PT_RUN_DIR may point at a reused one.
# Without this a re-run passes on last run's files even if this build never
# wrote a byte -- the classic stale-artifact false green.
rm -f "$BLANK" "$CANVAS" "$DONE" "$LOG" "$CONTROL_LOG"

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
    >"$CONTROL_LOG" 2>&1
control_rc=$?
set -e

# Three assertions, because "not 124" is not the same as "died for the reason
# this control exists to demonstrate".
#
# 124 is timeout's "still running", i.e. it launched successfully. If the binary
# survives without the library path, then either libgossamer is being resolved
# some other way or the rpath resolves in-tree -- and in both cases the positive
# run above no longer demonstrates what it claims.
if [ "$control_rc" -eq 124 ]; then
    die "control survived without LD_LIBRARY_PATH (rc=124); the positive result is vacuous"
fi
# rc=0 means it exited cleanly and early without the library. That is not a
# failure to launch, so it equally destroys the claim -- and a bare "not 124"
# test would have called it a PASS.
if [ "$control_rc" -eq 0 ]; then
    die "control exited 0 without LD_LIBRARY_PATH; it did not fail to launch"
fi
# And it must have died of THIS cause. Any other crash -- a missing GTK library,
# a segfault, a bad argument -- also yields a nonzero rc while proving nothing
# about libgossamer, which would make this control vacuous in the one direction
# an exit code cannot distinguish.
if ! command grep -Fq 'libgossamer' "$CONTROL_LOG"; then
    printf -- '--- %s ---\n' "$CONTROL_LOG"
    cat "$CONTROL_LOG" 2>/dev/null || true
    die "control failed (rc=$control_rc) but not over libgossamer; cause unproven"
fi
printf 'PASS: control died for want of libgossamer (rc=%s)\n' "$control_rc"

# ---------------------------------------------------------------------------
# Isolation. Run alongside scenario_host_headless.sh, which drives the same
# raster core with no webview. If that one writes its PNG and this one does not,
# the defect is in the webview or the bridge and provably not in paint_core.
# ---------------------------------------------------------------------------
printf 'PASS: Tier C -- paint-type launched, rendered and saved\n'
