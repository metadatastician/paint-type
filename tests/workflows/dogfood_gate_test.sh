#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW="$PROJECT_DIR/.github/workflows/dogfood-gate.yml"
LOCK="$PROJECT_DIR/.github/workflows/actions.lock"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/dogfood-gate-test-XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

if grep -Eiq 'a2ml' "$WORKFLOW" || grep -Fiq 'hyperpolymath/a2ml-ecosystem@main' "$LOCK"; then
  echo 'FAIL: retired A2ML gate or action remains in workflow or lockfile' >&2
  exit 1
fi

jobs="$(awk '/^jobs:/{inside=1; next} inside && /^  [a-z0-9-]+:/{print $1}' "$WORKFLOW")"
expected_jobs=$'k9-validate:\nempty-lint:\ngroove-check:\neclexiaiser-validate:\ndogfood-summary:'
if [ "$jobs" != "$expected_jobs" ]; then
  echo "FAIL: unexpected dogfood jobs: $jobs" >&2
  exit 1
fi

if ! bash "$PROJECT_DIR/scripts/check-lock-sync.sh" "$PROJECT_DIR/.github/workflows" > "$TEST_ROOT/lock-result" 2>&1; then
  echo 'FAIL: workflow actions are not synchronized with actions.lock' >&2
  cat "$TEST_ROOT/lock-result" >&2
  exit 1
fi
echo 'PASS: retired gate removed and remaining actions locked'

awk '
  /^      - name: Generate dogfooding scorecard$/ { found=1; next }
  found && /^        run: \|$/ { script=1; next }
  script && /^          / { print substr($0, 11); next }
  script && /^$/ { print; next }
  script { exit }
' "$WORKFLOW" > "$TEST_ROOT/scorecard.sh"

if [ ! -s "$TEST_ROOT/scorecard.sh" ]; then
  echo 'FAIL: scorecard script is missing' >&2
  exit 1
fi
bash -n "$TEST_ROOT/scorecard.sh"

write_fixture() {
  local fixture="$1"
  local directory="$2"
  case "$fixture" in
    k9) printf 'contract\n' > "$directory/contract.k9" ;;
    editorconfig) printf 'root = true\n' > "$directory/.editorconfig" ;;
    groove) mkdir -p "$directory/.well-known/groove"; printf '{}\n' > "$directory/.well-known/groove/manifest.json" ;;
    vsdb) printf 'backend = "VeriSimDB"\n' > "$directory/storage.toml" ;;
    eclexiaiser) printf 'budget = 1\n' > "$directory/eclexiaiser.toml" ;;
    a2ml) printf 'legacy\n' > "$directory/0-AI-MANIFEST.a2ml" ;;
    *) echo "Unknown fixture: $fixture" >&2; exit 1 ;;
  esac
}

run_case() {
  local name="$1"
  local fixtures="$2"
  local score="$3"
  local directory="$TEST_ROOT/$name"
  local fixture label status expected
  mkdir -p "$directory"
  for fixture in $fixtures; do
    write_fixture "$fixture" "$directory"
  done

  (cd "$directory" && GITHUB_STEP_SUMMARY="$directory/summary" bash "$TEST_ROOT/scorecard.sh")
  if ! grep -Fq "**Score: $score/5**" "$directory/summary"; then
    echo "FAIL: $name has incorrect score" >&2
    exit 1
  fi
  if grep -Eiq 'a2ml' "$directory/summary"; then
    echo "FAIL: $name still reports the retired format" >&2
    exit 1
  fi

  for fixture in k9 editorconfig groove vsdb eclexiaiser; do
    case "$fixture" in
      k9) label='K9 contracts'; expected=':x:' ;;
      editorconfig) label='.editorconfig'; expected=':x:' ;;
      groove) label='Groove endpoint'; expected=':ballot_box_with_check:' ;;
      vsdb) label='VeriSimDB integration'; expected=':ballot_box_with_check:' ;;
      eclexiaiser) label='eclexiaiser'; expected=':ballot_box_with_check:' ;;
    esac
    status="$expected"
    if [[ " $fixtures " == *" $fixture "* ]]; then
      status=':white_check_mark:'
    fi
    if ! grep -Fq "| $label | $status |" "$directory/summary"; then
      echo "FAIL: $name has incorrect $label status" >&2
      exit 1
    fi
  done
  echo "PASS: $name ($score/5)"
}

run_case empty '' 0
run_case legacy-manifest-only 'a2ml' 0
run_case k9-only 'k9' 1
run_case editorconfig-only 'editorconfig' 1
run_case groove-only 'groove' 1
run_case vsdb-only 'vsdb' 1
run_case eclexiaiser-only 'eclexiaiser' 1
run_case all-formats 'k9 editorconfig groove vsdb eclexiaiser' 5
run_case all-with-legacy-manifest 'k9 editorconfig groove vsdb eclexiaiser a2ml' 5
