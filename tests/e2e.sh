#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# RSR Standard E2E Test Template
#
# End-to-end tests validate the full pipeline: build → run → verify output.
# Customise this file for your project. Delete the examples that don't apply.
#
# Usage:
#   bash tests/e2e.sh
#   just e2e
#
# Merge requirements (STANDING): All 6 test categories must pass before merge:
#   P2P, E2E (this file), aspect, execution, lifecycle, benchmarks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

PASS=0
FAIL=0
SKIP=0

# ─── Colour helpers ──────────────────────────────────────────────────
green() { printf '\033[32m%s\033[0m\n' "$*"; }
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
yellow(){ printf '\033[33m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

# ─── Assertion helpers ───────────────────────────────────────────────

# check <label> <expected-substring> <actual>
check() {
    local name="$1" expected="$2" actual="$3"
    if echo "$actual" | grep -q "$expected"; then
        green "  PASS: $name"
        PASS=$((PASS + 1))
    else
        red "  FAIL: $name (expected '$expected', got '${actual:0:120}')"
        FAIL=$((FAIL + 1))
    fi
}

# skip <label> <reason>
skip_test() {
    yellow "  SKIP: $1 ($2)"
    SKIP=$((SKIP + 1))
}

fail_test() {
    red "  FAIL: $1"
    FAIL=$((FAIL + 1))
}

echo "═══════════════════════════════════════════════════════════════"
echo "  paint-type — End-to-End Tests"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# ─── Preflight ───────────────────────────────────────────────────────
bold "Preflight checks"

# TODO: Check that your binary/server is built
# Example:
# BINARY="$PROJECT_DIR/target/release/my-tool"
# if [ ! -f "$BINARY" ]; then
#     red "Binary not found at $BINARY — run 'just build' first"
#     exit 1
# fi
# green "  Binary found: $BINARY"

# TODO: Check dependencies
# command -v curl >/dev/null 2>&1 || { red "curl not found"; exit 1; }
# command -v jq >/dev/null 2>&1   || { red "jq not found"; exit 1; }

echo ""

# ═══════════════════════════════════════════════════════════════════════
# Section 1: Desktop shell (WebKitGTK) — open app, empty canvas, quit clean
# ═══════════════════════════════════════════════════════════════════════
# Builds the v0.3.0 webview shell (src/shell/, a GTK3 + WebKitGTK host on the
# same stack as gossamer/webview_gtk) and launches it headlessly under Xvfb in
# smoke mode. The shell creates the window + web view, loads the empty-canvas
# page, then auto-quits — emitting two markers the harness asserts. Skips
# cleanly where the webview toolkit or a headless display is unavailable (e.g.
# the macOS/Windows matrix legs), so it never produces a false failure.
bold "Section 1: Desktop shell — launch, empty canvas, quit clean"

SHELL_DIR="$PROJECT_DIR/src/shell"
GOSSAMER_DIR="$PROJECT_DIR/third_party/gossamer/src/interface/ffi"
GOSSAMER_LIB_DIR="$GOSSAMER_DIR/zig-out/lib"

if ! command -v zig >/dev/null 2>&1; then
    skip_test "desktop shell launch" "zig not found"
elif ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    skip_test "desktop shell launch" "webkit2gtk-4.1 not installed (webview toolkit absent)"
elif ! command -v xvfb-run >/dev/null 2>&1; then
    skip_test "desktop shell launch" "xvfb-run not found (no headless display)"
else
    if (
        cd "$GOSSAMER_DIR" &&
        zig build -Doptimize=ReleaseSafe &&
        export LIBRARY_PATH="$GOSSAMER_LIB_DIR${LIBRARY_PATH:+:$LIBRARY_PATH}" &&
        cd "$SHELL_DIR" &&
        zig build
    ) >/tmp/pt-shell-build.log 2>&1; then
        green "  PASS: shell builds (GTK3 + WebKitGTK)"
        PASS=$((PASS + 1))
        SHELL_BIN="$SHELL_DIR/zig-out/bin/paint-type-shell"
        SHELL_OUT=$(timeout 40 xvfb-run -a -s "-screen 0 1280x1024x24" \
            env PT_SHELL_SMOKE=1 \
                LD_LIBRARY_PATH="$GOSSAMER_LIB_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
                WEBKIT_DISABLE_COMPOSITING_MODE=1 \
                WEBKIT_DISABLE_DMABUF_RENDERER=1 \
                LIBGL_ALWAYS_SOFTWARE=1 \
                GDK_BACKEND=x11 \
            "$SHELL_BIN" 2>&1) || true
        check "shell opens window + empty canvas" "PT_SHELL: canvas-ready" "$SHELL_OUT"
        check "shell quits cleanly" "PT_SHELL: quit-clean" "$SHELL_OUT"
    else
        fail_test "shell build failed (see /tmp/pt-shell-build.log)"
        tail -5 /tmp/pt-shell-build.log || true
    fi
fi

# ═══════════════════════════════════════════════════════════════════════
# Section 2: Real-repository pipeline & scenario checks
# ═══════════════════════════════════════════════════════════════════════
# Executed only when running inside the full repository tree (kept separate
# from Section 1's isolated synthetic shell fixture in shell_e2e_harness_test.sh).
if [ -f "$PROJECT_DIR/src/interface/ffi/build.zig" ]; then
    echo ""
    bold "Section 2: FFI, harness, Groove manifest, and Rust scenario suite"

    if command -v zig >/dev/null 2>&1; then
        if (cd "$PROJECT_DIR" && zig build test) >/tmp/pt-root-zig-test.log 2>&1; then
            green "  PASS: root Zig build & tests"
            PASS=$((PASS + 1))
        else
            fail_test "root Zig tests failed (see /tmp/pt-root-zig-test.log)"
            tail -10 /tmp/pt-root-zig-test.log || true
        fi

        if (cd "$PROJECT_DIR/src/interface/ffi" && zig build && zig build test) >/tmp/pt-ffi-zig-test.log 2>&1; then
            green "  PASS: FFI Zig library build & tests"
            PASS=$((PASS + 1))
        else
            fail_test "FFI Zig build/tests failed (see /tmp/pt-ffi-zig-test.log)"
            tail -10 /tmp/pt-ffi-zig-test.log || true
        fi

        if bash "$PROJECT_DIR/tests/e2e/scenario_libpt_artifacts.sh" "$PROJECT_DIR" >/tmp/pt-scenario-libpt.log 2>&1; then
            green "  PASS: scenario_libpt_artifacts"
            PASS=$((PASS + 1))
        else
            fail_test "scenario_libpt_artifacts failed"
            tail -10 /tmp/pt-scenario-libpt.log || true
        fi
    else
        skip_test "Zig FFI & artifact scenarios" "zig not found"
    fi

    if bash "$PROJECT_DIR/tests/e2e/shell_e2e_harness_test.sh" >/tmp/pt-shell-harness-unit.log 2>&1; then
        green "  PASS: shell_e2e_harness_test"
        PASS=$((PASS + 1))
    else
        fail_test "shell_e2e_harness_test failed"
        tail -15 /tmp/pt-shell-harness-unit.log || true
    fi

    if bash "$PROJECT_DIR/scripts/validate_groove_manifest.sh" >/tmp/pt-groove-e2e.log 2>&1; then
        green "  PASS: validate_groove_manifest"
        PASS=$((PASS + 1))
    else
        fail_test "validate_groove_manifest failed"
        tail -10 /tmp/pt-groove-e2e.log || true
    fi

    if command -v cargo >/dev/null 2>&1; then
        for rust_scenario in \
            "$PROJECT_DIR/tests/e2e/scenario_pipeline_dogfood.sh" \
            "$PROJECT_DIR/tests/e2e/scenario_host_headless.sh" \
            "$PROJECT_DIR/tests/e2e/collaboration_session_test.sh"
        do
            sname="$(basename "$rust_scenario" .sh)"
            if bash "$rust_scenario" "$PROJECT_DIR" >"/tmp/pt-${sname}.log" 2>&1; then
                green "  PASS: $sname"
                PASS=$((PASS + 1))
            else
                fail_test "$sname failed"
                tail -20 "/tmp/pt-${sname}.log" || true
            fi
        done
    else
        skip_test "Rust E2E scenarios" "cargo not found"
    fi
fi

# ═══════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════"
printf "  Results: "
green "PASS=$PASS" | tr -d '\n'
echo -n "  "
if [ "$FAIL" -gt 0 ]; then red "FAIL=$FAIL" | tr -d '\n'; else echo -n "FAIL=0"; fi
echo -n "  "
if [ "$SKIP" -gt 0 ]; then yellow "SKIP=$SKIP"; else echo "SKIP=0"; fi
echo "═══════════════════════════════════════════════════════════════"

exit "$FAIL"
