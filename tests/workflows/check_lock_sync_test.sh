#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECKER="$PROJECT_DIR/scripts/check-lock-sync.sh"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/check-lock-sync-test-XXXXXX")"

PASS=0
FAIL=0
LAST_OUTPUT=""
LAST_STATUS=0
CASE_DIR=""

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

new_case() {
  CASE_DIR="$TEST_ROOT/$1"
  mkdir -p "$CASE_DIR"
}

write_workflow() {
  local name="$1"
  local content="$2"
  printf '%s\n' "$content" > "$CASE_DIR/$name"
}

write_lock() {
  local content="$1"
  printf '%s\n' "$content" > "$CASE_DIR/actions.lock"
}

setup_valid_fixture() {
  local name="$1"
  new_case "$name"

  write_workflow "main.yml" 'name: Main
on: push
jobs:
  action:
    runs-on: ubuntu-latest
    steps:
      - uses: "Acme/Build/action@v1" # subpaths and trailing comments are normalized
      - uses: ./local-action
  reusable:
    uses: acme/shared/.github/workflows/reusable.yaml@release'

  write_workflow "empty.yaml" 'name: Local only
on: push
jobs:
  local:
    runs-on: ubuntu-latest
    steps:
      - uses: $/local-action'

  write_lock "version: 'v0.0.2'
workflows:
    '.github/workflows/empty.yaml': []
    '.github/workflows/main.yml':
        - 'acme/build@v1'
        - 'acme/shared@release'
dependencies:
    'acme/build@v1': {}
    'acme/shared@release':
        uses:
            - 'acme/transitive@sha'
    'acme/transitive@sha': {}"
}

run_checker() {
  local workflows_dir="$1"
  LAST_OUTPUT="$(bash "$CHECKER" "$workflows_dir" 2>&1)"
  LAST_STATUS=$?
}

expect_pass() {
  local name="$1"
  local expected="$2"

  run_checker "$CASE_DIR"
  if [ "$LAST_STATUS" -ne 0 ]; then
    fail "$name (expected exit 0, got $LAST_STATUS)"
    printf '%s\n' "$LAST_OUTPUT" >&2
  elif [[ "$LAST_OUTPUT" != *"$expected"* ]]; then
    fail "$name (missing output: $expected)"
    printf '%s\n' "$LAST_OUTPUT" >&2
  else
    pass "$name"
  fi
}

expect_fail() {
  local name="$1"
  local expected="$2"

  run_checker "$CASE_DIR"
  if [ "$LAST_STATUS" -eq 0 ]; then
    fail "$name (expected a non-zero exit)"
    printf '%s\n' "$LAST_OUTPUT" >&2
  elif [[ "$LAST_OUTPUT" != *"$expected"* ]]; then
    fail "$name (missing output: $expected)"
    printf '%s\n' "$LAST_OUTPUT" >&2
  else
    pass "$name"
  fi
}

expect_rejection() {
  local name="$1"

  run_checker "$CASE_DIR"
  if [ "$LAST_STATUS" -eq 0 ]; then
    fail "$name (expected a non-zero exit)"
    printf '%s\n' "$LAST_OUTPUT" >&2
  else
    pass "$name"
  fi
}

test_valid_closed_lock() {
  setup_valid_fixture "valid"
  expect_pass "accepts a synchronized, transitively closed lock" \
    "every workflow file has a lockfile key"
}

test_missing_lockfile() {
  new_case "missing-lockfile"
  write_workflow "main.yml" 'name: Main'
  expect_fail "rejects a missing lockfile" "FATAL: no lockfile"
}

test_no_workflows() {
  new_case "no-workflows"
  write_lock "version: 'v0.0.2'
workflows:
dependencies:"
  expect_rejection "rejects a directory with no workflows"
}

test_missing_step_ref() {
  setup_valid_fixture "missing-step"
  sed -i 's|Acme/Build/action@v1|Acme/Build/action@v2|' "$CASE_DIR/main.yml"
  expect_fail "rejects an unlocked step-level action" \
    "step-level refs missing from the lockfile: Acme/Build@v2"
}

test_ref_locked_under_wrong_workflow() {
  setup_valid_fixture "wrong-workflow"
  sed -i "/^        - 'acme\/build@v1'$/d" "$CASE_DIR/actions.lock"
  sed -i "/empty.yaml': \[\]/c\    '.github/workflows/empty.yaml':\n        - 'acme/build@v1'" "$CASE_DIR/actions.lock"
  expect_fail "requires refs under the workflow that uses them" \
    "step-level refs missing from the lockfile: Acme/Build@v1"
}

test_unonboarded_workflow_with_ref() {
  setup_valid_fixture "unonboarded"
  write_workflow "new.yml" 'name: New
on: push
jobs:
  action:
    runs-on: ubuntu-latest
    steps:
      - uses: acme/new/action@v1'
  expect_fail "identifies a workflow with refs but no lock key" \
    "not onboarded: no lockfile entry for this path"
}

test_missing_reusable_workflow_ref() {
  setup_valid_fixture "missing-reusable"
  sed -i 's|@release|@next|' "$CASE_DIR/main.yml"
  expect_fail "rejects an unlocked reusable-workflow ref" \
    "job-level reusable refs missing from the lockfile: acme/shared@next"
}

test_stale_lock_entry() {
  setup_valid_fixture "stale-entry"
  sed -i "/        - 'acme\/shared@release'/a\        - 'acme/old@v1'" "$CASE_DIR/actions.lock"
  printf "    'acme/old@v1': {}\n" >> "$CASE_DIR/actions.lock"
  expect_fail "rejects a stale per-workflow lock entry" \
    "stale lockfile entries, no uses: references them: acme/old@v1"
}

test_deleted_workflow_entry() {
  setup_valid_fixture "deleted-workflow"
  sed -i "/^dependencies:/i\    '.github/workflows/deleted.yml': []" "$CASE_DIR/actions.lock"
  expect_fail "rejects a lock key for a deleted workflow" \
    "lockfile entry for a workflow file that does not exist"
}

test_unlisted_zero_uses_workflow() {
  setup_valid_fixture "unlisted-empty"
  sed -i "/empty.yaml': \[\]/d" "$CASE_DIR/actions.lock"
  expect_fail "rejects an unlisted workflow with no external refs" \
    "FAIL actions.lock: UNLISTED WORKFLOWS"
}

test_missing_direct_dependency() {
  setup_valid_fixture "missing-direct-dependency"
  sed -i "/acme\/build@v1': {}/d" "$CASE_DIR/actions.lock"
  expect_fail "rejects a workflow ref with no dependency record" \
    "acme/build@v1"
}

test_missing_transitive_dependency() {
  setup_valid_fixture "missing-transitive-dependency"
  sed -i "/acme\/transitive@sha': {}/d" "$CASE_DIR/actions.lock"
  expect_fail "rejects a dangling nested dependency" \
    "named by: dependencies:acme/shared@release"
}

test_owner_repo_case_is_insensitive() {
  setup_valid_fixture "owner-case"
  sed -i "s|'acme/build@v1': {}|'ACME/BUILD@v1': {}|" "$CASE_DIR/actions.lock"
  expect_pass "compares owner/repository names case-insensitively" \
    "0 dangling edges"
}

test_ref_case_is_sensitive() {
  setup_valid_fixture "ref-case"
  sed -i 's|action@v1|action@V1|' "$CASE_DIR/main.yml"
  expect_fail "compares refs case-sensitively" "Acme/Build@V1"
}

test_local_actions_are_ignored() {
  new_case "local-actions"
  write_workflow "local.yml" 'name: Local actions
on: push
jobs:
  local:
    runs-on: ubuntu-latest
    steps:
      - uses: ./relative/action
      - uses: $/rewritten/action'
  write_lock "version: 'v0.0.2'
workflows:
    '.github/workflows/local.yml': []
dependencies:"
  expect_pass "ignores both supported local-action forms" \
    "every uses: is locked"
}

test_unreferenced_dependency_is_nonfatal() {
  setup_valid_fixture "unreferenced-dependency"
  printf "    'acme/unused@v1': {}\n" >> "$CASE_DIR/actions.lock"
  expect_pass "allows an unreferenced dependency record" \
    "1 dependencies: record(s) are unreferenced"
}

test_repository_lock() {
  run_checker "$PROJECT_DIR/.github/workflows"
  if [ "$LAST_STATUS" -eq 0 ]; then
    pass "accepts the repository's checked-in workflow lock"
  else
    fail "accepts the repository's checked-in workflow lock"
    printf '%s\n' "$LAST_OUTPUT" >&2
  fi
}

test_gate_structure() {
  local gate="$PROJECT_DIR/.github/workflows/lock-sync-gate.yml"
  local name="keeps the lock-sync gate independent of external actions"

  if [ ! -x "$CHECKER" ]; then
    fail "$name (checker is not executable)"
  elif grep -Eq '^[[:space:]]*-?[[:space:]]*uses:' "$gate"; then
    fail "$name (gate contains a uses: directive)"
  elif grep -Eq '^[[:space:]]*paths(-ignore)?:' "$gate"; then
    fail "$name (gate contains a path filter)"
  elif ! grep -Fq './scripts/check-lock-sync.sh' "$gate"; then
    fail "$name (gate does not invoke the checker)"
  elif ! grep -Eq '^[[:space:]]*pull_request:' "$gate" \
       || ! grep -Eq '^[[:space:]]*push:' "$gate"; then
    fail "$name (gate does not cover pull requests and pushes)"
  else
    pass "$name"
  fi
}

test_valid_closed_lock
test_missing_lockfile
test_no_workflows
test_missing_step_ref
test_ref_locked_under_wrong_workflow
test_unonboarded_workflow_with_ref
test_missing_reusable_workflow_ref
test_stale_lock_entry
test_deleted_workflow_entry
test_unlisted_zero_uses_workflow
test_missing_direct_dependency
test_missing_transitive_dependency
test_owner_repo_case_is_insensitive
test_ref_case_is_sensitive
test_local_actions_are_ignored
test_unreferenced_dependency_is_nonfatal
test_repository_lock
test_gate_structure

printf '\nResults: %d passed; %d failed\n' "$PASS" "$FAIL"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
