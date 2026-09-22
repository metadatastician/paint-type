#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
E2E_SCRIPT="$PROJECT_DIR/tests/e2e.sh"
SHELL_SOURCE="$PROJECT_DIR/src/shell/main.zig"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/shell-e2e-harness-test-XXXXXX")"

PASS=0
FAIL=0
FIXTURE=""
FAKE_BIN=""
CALL_LOG=""
LAST_OUTPUT=""
LAST_STATUS=0

cleanup() {
  rm -rf "$TEST_ROOT"
}
trap cleanup EXIT

pass() {
  printf 'PASS: %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

assert_status() {
  local name="$1"
  local expected="$2"

  if [ "$LAST_STATUS" -eq "$expected" ]; then
    pass "$name"
  else
    fail "$name (expected exit $expected, got $LAST_STATUS)"
    printf '%s\n' "$LAST_OUTPUT" >&2
  fi
}

assert_contains() {
  local name="$1"
  local haystack="$2"
  local needle="$3"

  if [[ "$haystack" == *"$needle"* ]]; then
    pass "$name"
  else
    fail "$name (missing: $needle)"
    printf '%s\n' "$haystack" >&2
  fi
}

assert_file_contains() {
  local name="$1"
  local file="$2"
  local needle="$3"

  if grep -Fq "$needle" "$file"; then
    pass "$name"
  else
    fail "$name (missing from $file: $needle)"
    sed -n '1,120p' "$file" >&2
  fi
}

assert_file_not_contains() {
  local name="$1"
  local file="$2"
  local needle="$3"

  if grep -Fq "$needle" "$file"; then
    fail "$name (unexpected content in $file: $needle)"
    sed -n '1,120p' "$file" >&2
  else
    pass "$name"
  fi
}

write_executable() {
  local path="$1"
  local content="$2"

  printf '%s\n' "$content" > "$path"
  chmod +x "$path"
}

setup_fixture() {
  local name="$1"

  FIXTURE="$TEST_ROOT/$name"
  FAKE_BIN="$FIXTURE/fake-bin"
  CALL_LOG="$FIXTURE/calls.log"
  mkdir -p \
    "$FAKE_BIN" \
    "$FIXTURE/tests" \
    "$FIXTURE/src/shell" \
    "$FIXTURE/third_party/gossamer/src/interface/ffi"
  : > "$CALL_LOG"

  cp "$E2E_SCRIPT" "$FIXTURE/tests/e2e.sh"
  sed -i "s|/tmp/pt-shell-build.log|$FIXTURE/pt-shell-build.log|g" \
    "$FIXTURE/tests/e2e.sh"

  write_executable "$FAKE_BIN/zig" '#!/usr/bin/env bash
set -uo pipefail
printf "zig|%s|%s|LIBRARY_PATH=%s\n" "$PWD" "$*" "${LIBRARY_PATH-<unset>}" >> "$CALL_LOG"
if [ "${FAIL_ZIG_DIR-}" = "$PWD" ]; then
  printf "injected zig failure in %s\n" "$PWD" >&2
  exit 42
fi'

  write_executable "$FAKE_BIN/pkg-config" '#!/usr/bin/env bash
exit 0'

  write_executable "$FAKE_BIN/xvfb-run" '#!/usr/bin/env bash
exit 0'

  write_executable "$FAKE_BIN/timeout" '#!/usr/bin/env bash
set -uo pipefail
printf "timeout|%s\n" "$*" >> "$CALL_LOG"
printf "PT_SHELL: canvas-ready\nPT_SHELL: quit-clean\n"'
}

run_harness_with_paths() {
  local library_path="$1"
  local runtime_path="$2"
  local fail_dir="${3:-}"

  LAST_OUTPUT="$(env \
    PATH="$FAKE_BIN:/usr/bin:/bin" \
    CALL_LOG="$CALL_LOG" \
    LIBRARY_PATH="$library_path" \
    LD_LIBRARY_PATH="$runtime_path" \
    FAIL_ZIG_DIR="$fail_dir" \
    bash "$FIXTURE/tests/e2e.sh" 2>&1)"
  LAST_STATUS=$?
}

run_harness_without_paths() {
  LAST_OUTPUT="$(env -u LIBRARY_PATH -u LD_LIBRARY_PATH \
    PATH="$FAKE_BIN:/usr/bin:/bin" \
    CALL_LOG="$CALL_LOG" \
    bash "$FIXTURE/tests/e2e.sh" 2>&1)"
  LAST_STATUS=$?
}

test_build_order_and_path_propagation() {
  local gossamer_dir
  local gossamer_lib
  local shell_dir

  setup_fixture "paths"
  gossamer_dir="$FIXTURE/third_party/gossamer/src/interface/ffi"
  gossamer_lib="$gossamer_dir/zig-out/lib"
  shell_dir="$FIXTURE/src/shell"
  run_harness_with_paths "/existing/build" "/existing/runtime"

  assert_status "successful mocked shell flow exits zero" 0
  assert_contains "successful mocked shell flow records all assertions" \
    "$LAST_OUTPUT" "PASS=3"
  assert_file_contains "builds Gossamer in its FFI directory first" "$CALL_LOG" \
    "zig|$gossamer_dir|build -Doptimize=ReleaseSafe|LIBRARY_PATH=/existing/build"
  assert_file_contains "adds the Gossamer library while building the shell" "$CALL_LOG" \
    "zig|$shell_dir|build|LIBRARY_PATH=$gossamer_lib:/existing/build"
  assert_file_contains "adds the Gossamer library while launching the shell" "$CALL_LOG" \
    "LD_LIBRARY_PATH=$gossamer_lib:/existing/runtime"

  local first_call
  first_call="$(sed -n '1p' "$CALL_LOG")"
  if [[ "$first_call" == "zig|$gossamer_dir|"* ]]; then
    pass "Gossamer build precedes the shell build"
  else
    fail "Gossamer build precedes the shell build"
    sed -n '1,120p' "$CALL_LOG" >&2
  fi
}

test_unset_paths_do_not_add_empty_segments() {
  local gossamer_dir
  local gossamer_lib

  setup_fixture "unset-paths"
  gossamer_dir="$FIXTURE/third_party/gossamer/src/interface/ffi"
  gossamer_lib="$gossamer_dir/zig-out/lib"
  run_harness_without_paths

  assert_status "unset library paths remain supported" 0
  assert_file_contains "shell build path has no trailing empty segment" "$CALL_LOG" \
    "LIBRARY_PATH=$gossamer_lib"
  assert_file_contains "runtime path has no trailing empty segment" "$CALL_LOG" \
    "LD_LIBRARY_PATH=$gossamer_lib"
  if grep -Fq "$gossamer_lib:" "$CALL_LOG"; then
    fail "unset library paths do not create a trailing colon"
    sed -n '1,120p' "$CALL_LOG" >&2
  else
    pass "unset library paths do not create a trailing colon"
  fi
}

test_shell_build_failure_is_reported() {
  local shell_dir

  setup_fixture "shell-build-failure"
  shell_dir="$FIXTURE/src/shell"
  run_harness_with_paths "" "" "$shell_dir"

  assert_status "shell build failure makes the harness fail" 1
  assert_contains "shell build failure is named" "$LAST_OUTPUT" \
    "FAIL: shell build failed"
  assert_file_not_contains "shell build failure prevents launch" "$CALL_LOG" \
    "timeout|"
}

test_gossamer_build_failure_is_reported() {
  local gossamer_dir

  setup_fixture "gossamer-build-failure"
  gossamer_dir="$FIXTURE/third_party/gossamer/src/interface/ffi"
  run_harness_with_paths "" "" "$gossamer_dir"

  assert_status "Gossamer build failure makes the harness fail" 1
  assert_contains "Gossamer build failure is named" "$LAST_OUTPUT" \
    "FAIL: shell build failed"
  assert_file_not_contains "Gossamer failure prevents the shell build" "$CALL_LOG" \
    "zig|$FIXTURE/src/shell|"
  assert_file_not_contains "Gossamer failure prevents launch" "$CALL_LOG" \
    "timeout|"
}

test_zig_source_regressions() {
  if grep -Fq 'callconv(.c)' "$SHELL_SOURCE" \
     && ! grep -Fq 'callconv(.C)' "$SHELL_SOURCE"; then
    pass "GTK callback uses the current Zig C calling convention"
  else
    fail "GTK callback uses the current Zig C calling convention"
  fi

  if grep -Fq 'gossamer_create_ex failed: {s}\n", .{std.mem.span(err)}' "$SHELL_SOURCE"; then
    pass "create errors become sentinel slices before formatting"
  else
    fail "create errors become sentinel slices before formatting"
  fi

  if grep -Fq 'gossamer_create_ex returned null handle' "$SHELL_SOURCE" \
     && ! grep -Fq 'err != null orelse' "$SHELL_SOURCE"; then
    pass "null create errors use an explicit fallback message"
  else
    fail "null create errors use an explicit fallback message"
  fi

  if grep -Fq 'gossamer_load_html failed: {s}\n", .{std.mem.span(err)}' "$SHELL_SOURCE"; then
    pass "load errors become sentinel slices before formatting"
  else
    fail "load errors become sentinel slices before formatting"
  fi

  if grep -Fq 'gossamer_load_html failed: unknown error' "$SHELL_SOURCE" \
     && ! grep -Fq 'err != null orelse' "$SHELL_SOURCE"; then
    pass "null load errors use an explicit fallback message"
  else
    fail "null load errors use an explicit fallback message"
  fi
}

test_build_order_and_path_propagation
test_unset_paths_do_not_add_empty_segments
test_shell_build_failure_is_reported
test_gossamer_build_failure_is_reported
test_zig_source_regressions

printf '\nResults: %d passed; %d failed\n' "$PASS" "$FAIL"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
