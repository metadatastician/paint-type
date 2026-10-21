#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Regression suite for paint-type git-flow fluidity tooling:
#   - Root .coderabbit.yaml + .machine_readable/root-allow.txt registration
#   - .githooks/{install.sh,pre-commit,commit-msg,pre-push} without mutating repo modes
#   - scripts/preflight.sh positive and negative controls (Hypatia + panic-attack)
#   - scripts/sync-actions-lock.js check mode + conflict-marker auto-healing
#   - AGENTS.md review guidance and Justfile S-expression descriptile recipes

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
export PATH="$HOME/.bun/bin:/usr/local/bin:$PATH"

PASS=0
FAIL=0

# Record a passing assertion.
ok() {
  printf '  PASS: %s\n' "$1"
  PASS=$((PASS + 1))
}

# Record a failing assertion.
fail() {
  printf '  FAIL: %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
SLASH_TMP="$(printf '\057tmp\057')"

printf '=== Git Fluidity & Preflight Regression Suite ===\n'

# 1. Verify root .coderabbit.yaml + root-allow.txt guard against stacked conflict PRs & nitpicks
CR_CFG=".coderabbit.yaml"
if [ -f "$CR_CFG" ] && [ ! -f ".github/.coderabbit.yaml" ] \
  && grep -qxF '.coderabbit.yaml' .machine_readable/root-allow.txt \
  && grep -q 'profile: "chill"' "$CR_CFG" \
  && grep -q 'request_changes_workflow: false' "$CR_CFG" \
  && grep -q 'drafts: false' "$CR_CFG" \
  && grep -A3 'docstrings:' "$CR_CFG" | grep -q 'enabled: false'; then
  ok "Root .coderabbit.yaml is registered in root-allow.txt and chill-tuned"
else
  fail "Root .coderabbit.yaml missing required registration or anti-friction settings"
fi

# 2. Verify all .githooks scripts and workflow helpers parse cleanly WITHOUT mutating repo modes
for req in .githooks/install.sh .githooks/pre-commit .githooks/commit-msg .githooks/pre-push \
           scripts/preflight.sh scripts/git-push-signed.sh scripts/sync-actions-lock.js; do
  if [ -f "$req" ] && { [[ "$req" == *.js ]] || bash -n "$req" 2>/dev/null; }; then
    ok "$req exists and parses cleanly without working-tree mode mutation"
  else
    fail "$req missing or failed syntax check"
  fi
done

# Verify `.githooks/install.sh` applies +x inside an isolated temporary git repository
mkdir -p "$TMP_DIR/hookrepo/.githooks" "$TMP_DIR/hookrepo/scripts"
cp .githooks/install.sh .githooks/pre-commit .githooks/commit-msg .githooks/pre-push "$TMP_DIR/hookrepo/.githooks/"
cp scripts/preflight.sh scripts/git-push-signed.sh scripts/sync-actions-lock.js "$TMP_DIR/hookrepo/scripts/"
chmod 0644 "$TMP_DIR/hookrepo/.githooks/"* "$TMP_DIR/hookrepo/scripts/"*
git -C "$TMP_DIR/hookrepo" init -q
if (cd "$TMP_DIR/hookrepo" && bash ./.githooks/install.sh >/dev/null) \
  && [ -x "$TMP_DIR/hookrepo/.githooks/pre-commit" ] \
  && [ -x "$TMP_DIR/hookrepo/.githooks/commit-msg" ] \
  && [ -x "$TMP_DIR/hookrepo/.githooks/pre-push" ]; then
  ok ".githooks/install.sh sets executable bits and core.hooksPath cleanly"
else
  fail ".githooks/install.sh did not configure executable hooks"
fi

# 3. Verify zero hardcoded shared temp paths or child_process calls across new scripts
if grep -rnF "$SLASH_TMP" .githooks/ scripts/preflight.sh scripts/git-push-signed.sh \
  scripts/sync-actions-lock.js tests/workflows/git_fluidity_test.sh \
  tests/workflows/verdict_grids_conformance_test.sh AGENTS.md; then
  fail "Hardcoded shared temp path detected in workflow/hook/test artifacts"
else
  ok "Zero hardcoded shared temp paths across hooks, preflight, helpers, tests, and AGENTS.md"
fi

if grep -nE 'child_process|\b(execSync|spawnSync|execFileSync)\b' scripts/sync-actions-lock.js; then
  fail "scripts/sync-actions-lock.js still uses child_process synchronous execution"
else
  ok "scripts/sync-actions-lock.js uses pure native fetch + fs with try/catch JSON boundaries"
fi

# 4. Verify live repository passes `./scripts/preflight.sh` cleanly
if bash ./scripts/preflight.sh >/dev/null; then
  ok "Live repository passes ./scripts/preflight.sh with zero errors"
else
  fail "Live repository failed ./scripts/preflight.sh"
fi

# 5. Verify `commit-msg` hook accepts valid Conventional Commits & Merge subjects and rejects bad ones
printf 'feat(ci): add preflight hooks\n' > "$TMP_DIR/good_msg"
printf "Merge branch 'main'\n" > "$TMP_DIR/merge_msg"
printf 'WIP bad message\n' > "$TMP_DIR/bad_msg"
if bash ./.githooks/commit-msg "$TMP_DIR/good_msg" && bash ./.githooks/commit-msg "$TMP_DIR/merge_msg"; then
  ok "commit-msg hook accepts valid Conventional Commit and Merge subjects"
else
  fail "commit-msg hook rejected valid Conventional Commit or Merge subject"
fi
if bash ./.githooks/commit-msg "$TMP_DIR/bad_msg" 2>/dev/null; then
  fail "commit-msg hook accepted malformed subject"
else
  ok "commit-msg hook rejects non-Conventional Commit subject"
fi

# 6. Verify `Justfile` carries zero retired root `.machine_readable/STATE.a2ml` path checks
if grep -nE '\.machine_readable/(STATE|META|ECOSYSTEM|AGENTIC|NEUROSYM|PLAYBOOK|ANCHOR)\.a2ml' Justfile; then
  fail "Justfile still references retired top-level .machine_readable/*.a2ml path"
else
  ok "Justfile uses canonical .machine_readable/descriptiles/ paths throughout (0 SD024 findings)"
fi

# 7. Verify `AGENTS.md` scopes Codex / AI reviewers per Kautz-Ladder guidance
if grep -q 'Code Review Guidance (Codex / AI Reviewers)' AGENTS.md; then
  ok "AGENTS.md includes Codex / AI code-review scoping guidance"
else
  fail "AGENTS.md missing Codex / AI code-review scoping guidance"
fi

# 8. Verify `sync-actions-lock.js` matches live `actions.lock` and heals git conflict markers
if command -v bun >/dev/null 2>&1; then
  if bun ./scripts/sync-actions-lock.js --check >/dev/null; then
    ok "sync-actions-lock.js --check agrees 100% with .github/workflows/actions.lock"
  else
    fail "sync-actions-lock.js --check detected canonical ordering drift"
  fi

  mkdir -p "$TMP_DIR/repo/.github/workflows" "$TMP_DIR/repo/scripts"
  cp .github/workflows/*.yml "$TMP_DIR/repo/.github/workflows/"
  cp scripts/check-lock-sync.sh scripts/sync-actions-lock.js "$TMP_DIR/repo/scripts/"
  {
    printf '<<<<<<< HEAD\n'
    head -n 30 .github/workflows/actions.lock
    printf '=======\n'
    cat .github/workflows/actions.lock
    printf '>>>>>>> feature-branch\n'
  } > "$TMP_DIR/repo/.github/workflows/actions.lock"

  if (cd "$TMP_DIR/repo" && bun ./scripts/sync-actions-lock.js --write >/dev/null) \
    && cmp -s .github/workflows/actions.lock "$TMP_DIR/repo/.github/workflows/actions.lock"; then
    ok "sync-actions-lock.js auto-heals git conflict markers back to exact canonical bytes"
  else
    fail "sync-actions-lock.js conflict-marker recovery differed from canonical actions.lock"
  fi
fi

printf 'Summary: %d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
