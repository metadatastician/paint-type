#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Local CI and governance preflight gate for paint-type.
# Catches workflow, lockfile, allowlist, runtime-policy, invisible-Unicode,
# DEED/K9, and docstring issues in <2s before burning GitHub Actions runs or
# dismissing stale CODEOWNERS approvals on push.
#
# Usage:
#   ./scripts/preflight.sh            # Check working tree + branch delta vs origin/main
#   ./scripts/preflight.sh --staged   # Check staged index changes (pre-commit mode)
#   ./scripts/preflight.sh --push     # Check commits ahead of origin/main (pre-push mode)

set -euo pipefail

MODE="${1:---all}"
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

# Prepend common local Bun / tool locations when invoked inside non-login hooks.
export PATH="$HOME/.bun/bin:/tmp/tools-bin:/usr/local/bin:$PATH"

ERRORS=0
WARNINGS=0

# Record a hard gate failure and print an actionable diagnostic to stderr.
fail_gate() {
  printf '[preflight] FAIL: %s\n' "$1" >&2
  ERRORS=$((ERRORS + 1))
}

# Record a non-blocking warning diagnostic to stderr.
warn_gate() {
  printf '[preflight] WARN: %s\n' "$1" >&2
  WARNINGS=$((WARNINGS + 1))
}

# Print a passing gate line to stdout.
pass_gate() {
  printf '[preflight] OK: %s\n' "$1"
}

# Collect repository-relative paths touched in the requested mode.
collect_changed_files() {
  case "$MODE" in
    --staged)
      git -c core.quotePath=false diff --cached --name-only --diff-filter=ACMR 2>/dev/null || true
      ;;
    --push)
      if git rev-parse --verify origin/main >/dev/null 2>&1; then
        git -c core.quotePath=false diff --name-only --diff-filter=ACMR origin/main...HEAD 2>/dev/null || true
      else
        git -c core.quotePath=false diff --name-only --diff-filter=ACMR HEAD~1..HEAD 2>/dev/null || true
      fi
      ;;
    *)
      {
        git -c core.quotePath=false diff --name-only --diff-filter=ACMR HEAD 2>/dev/null || true
        git -c core.quotePath=false ls-files --others --exclude-standard 2>/dev/null || true
        if git rev-parse --verify origin/main >/dev/null 2>&1; then
          git -c core.quotePath=false diff --name-only --diff-filter=ACMR origin/main...HEAD 2>/dev/null || true
        fi
      } | awk 'NF && !seen[$0]++'
      ;;
  esac
}

# Collect newly added file paths relative to HEAD / origin/main for freeze gates.
collect_added_files() {
  case "$MODE" in
    --staged)
      git -c core.quotePath=false diff --cached --name-only --diff-filter=A 2>/dev/null || true
      ;;
    *)
      {
        git -c core.quotePath=false ls-files --others --exclude-standard 2>/dev/null || true
        if git rev-parse --verify origin/main >/dev/null 2>&1; then
          git -c core.quotePath=false diff --name-only --diff-filter=A origin/main...HEAD 2>/dev/null || true
        fi
      } | awk 'NF && !seen[$0]++'
      ;;
  esac
}

# Gate 1: Verify .github/workflows/actions.lock is closed and synchronized.
# Prefers `./scripts/check-lock-sync.sh` when GNU gawk is present and falls back
# seamlessly to `bun ./scripts/sync-actions-lock.js --verify` on BSD awk / mawk.
check_actions_lock() {
  if command -v gawk >/dev/null 2>&1 && [ -x ./scripts/check-lock-sync.sh ]; then
    if ./scripts/check-lock-sync.sh >/dev/null; then
      pass_gate "actions.lock synchronized and transitively closed (gawk)"
    else
      fail_gate "actions.lock out of sync with .github/workflows/*.yml (run: just sync-lock)"
    fi
  elif command -v bun >/dev/null 2>&1 && [ -f ./scripts/sync-actions-lock.js ]; then
    if bun ./scripts/sync-actions-lock.js --verify >/dev/null; then
      pass_gate "actions.lock synchronized and transitively closed (Bun)"
    else
      fail_gate "actions.lock out of sync with .github/workflows/*.yml (run: just sync-lock)"
    fi
  else
    warn_gate "neither gawk nor bun found; skipped local actions.lock check"
  fi
}

# Gate 2: Validate Hypatia SARIF workflow rules and runtime policy on modified workflows.
check_changed_workflows() {
  local changed="$1"
  local wf_checked=0
  local file

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    case "$file" in
      .github/workflows/*.yml|.github/workflows/*.yaml) ;;
      *) continue ;;
    esac
    [ -f "$file" ] || continue
    wf_checked=$((wf_checked + 1))

    if grep -nE 'secrets\.GITHUB_TOKEN' "$file" >&2; then
      fail_gate "$file uses secrets.GITHUB_TOKEN (use \${{ github.token }} per Hypatia RE001)"
    fi
    if grep -nE '\|\|[[:space:]]*true\b' "$file" >&2; then
      fail_gate "$file masks command exit codes with '|| true' (Hypatia RE005)"
    fi
    if grep -nE '^[[:space:]]*[^#].*(/tmp/[A-Za-z0-9_.-]+)' "$file" >&2; then
      fail_gate "$file references a literal /tmp path (use \$RUNNER_TEMP)"
    fi
    if grep -nE '^[[:space:]]*[^#].*\b(python3?|node|npm|pnpm|yarn)\b' "$file" >&2; then
      fail_gate "$file invokes a non-preferred workflow runtime (use bash, jq, and bun)"
    fi

    # Verify every job declaring `runs-on:` also declares `timeout-minutes:`.
    if ! awk '
      /^jobs:[[:space:]]*$/ { in_jobs = 1; next }
      in_jobs && /^[^[:space:]#]/ { in_jobs = 0 }
      !in_jobs { next }
      /^  [A-Za-z0-9_-]+:[[:space:]]*$/ {
        if (job != "" && has_runs && !has_timeout) {
          printf "  missing timeout-minutes on job %s\n", job > "/dev/stderr"
          bad = 1
        }
        job = $1; sub(/:$/, "", job)
        has_runs = 0; has_timeout = 0
        next
      }
      /^    runs-on:/ { has_runs = 1 }
      /^    timeout-minutes:/ { has_timeout = 1 }
      END {
        if (job != "" && has_runs && !has_timeout) {
          printf "  missing timeout-minutes on job %s\n", job > "/dev/stderr"
          bad = 1
        }
        exit(bad ? 1 : 0)
      }
    ' "$file"; then
      fail_gate "$file has job(s) without explicit timeout-minutes (Hypatia WH006)"
    fi

    # Verify every external action `uses:` in modified workflows is pinned to a 40-hex SHA.
    if ! awk '
      /^[[:space:]]*#/ { next }
      {
        line = $0
        sub(/[[:space:]]+#.*$/, "", line)
        if (line ~ /^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*[^[:space:]]+/) {
          ref = line
          sub(/^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*/, "", ref)
          sub(/[[:space:]]+.*$/, "", ref)
          gsub(/^["\x27]|["\x27]$/, "", ref)
          if (ref ~ /^\.\// || ref ~ /^\$\//) next
          at = index(ref, "@")
          sha = substr(ref, at + 1)
          if (at == 0 || length(sha) != 40 || sha !~ /^[0-9a-f]+$/) {
            printf "  unpinned uses at line %d: %s\n", NR, ref > "/dev/stderr"
            bad = 1
          }
        }
      }
      END { exit(bad ? 1 : 0) }
    ' "$file"; then
      fail_gate "$file contains unpinned third-party action ref (Hypatia WH001)"
    fi
  done <<< "$changed"

  pass_gate "workflow hygiene verified ($wf_checked modified workflow file(s))"
}

# Gate 3: Enforce banned file types, D43 .a2ml freeze, and single JS lockfile.
check_language_and_freeze_policies() {
  local changed="$1"
  local added="$2"
  local file

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    case "$file" in
      third_party/*) continue ;;
      *.ts|*.tsx) fail_gate "TypeScript file disallowed ($file); use ReScript/AffineScript or plain JS under Bun" ;;
      *.go)       fail_gate "Go file disallowed ($file); use Rust" ;;
      *.py)       fail_gate "Python source file disallowed ($file); use Rust, Julia, or Bun JS" ;;
      Makefile|*/Makefile|*.mk) fail_gate "Makefile disallowed ($file); use Justfile" ;;
      Dockerfile|*/Dockerfile)  fail_gate "Dockerfile disallowed ($file); use Containerfile with Podman" ;;
    esac
  done <<< "$changed"

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    case "$file" in
      *.a2ml)
        fail_gate "New .a2ml file disallowed under ruling D43 ($file); author .deed or .k9.ncl instead"
        ;;
    esac
  done <<< "$added"

  local pm_count=0
  { [ -f bun.lockb ] || [ -f bun.lock ]; } && pm_count=$((pm_count + 1)) || true
  { [ -f deno.lock ] || [ -f deno.json ] || [ -f deno.jsonc ]; } && pm_count=$((pm_count + 1)) || true
  [ -f pnpm-lock.yaml ] && pm_count=$((pm_count + 1)) || true
  [ -f package-lock.json ] && pm_count=$((pm_count + 1)) || true
  if [ "$pm_count" -gt 1 ]; then
    fail_gate "Multiple JS package-manager lockfiles present ($pm_count); prefer Bun first"
  else
    pass_gate "language, package-manager (Bun > Deno), and D43 .a2ml freeze checks passed"
  fi
}

# Gate 4: Scan changed text files for zero-width / BiDi invisible characters (Empty-Linter parity).
check_invisible_unicode() {
  local changed="$1"
  local file
  local checked=0

  if ! command -v perl >/dev/null 2>&1; then
    warn_gate "perl not installed; skipping local invisible-Unicode check"
    return 0
  fi

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    [ -f "$file" ] || continue
    case "$file" in
      *.png|*.jpg|*.jpeg|*.gif|*.bmp|*.ico|*.webp|*.wasm|*.pdf|*.woff|*.woff2|*.zip|*.gz|*.tar|*.spv)
        continue
        ;;
    esac
    checked=$((checked + 1))
    if perl -CSD -ne 'exit 10 if /[\x{200B}-\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{FEFF}]/' "$file"; then
      :
    else
      if [ "$?" -eq 10 ]; then
        fail_gate "$file contains zero-width, BOM, or BiDi control characters"
      fi
    fi
  done <<< "$changed"

  pass_gate "invisible-character check passed ($checked text file(s))"
}

# Gate 5: Validate DEED SHA-256 fields and coordination.k9 ABNF v1.2 grammar.
check_deed_and_k9() {
  if [ -f paint-type_chora.deed ] && [ -f scripts/deed-field.sh ]; then
    local csha gsha
    csha="$(bash scripts/deed-field.sh paint-type_chora.deed canon criteria-sha256)"
    gsha="$(bash scripts/deed-field.sh paint-type_chora.deed canon gates-sha256)"
    if [[ "$csha" =~ ^[0-9a-f]{64}$ ]] && [[ "$gsha" =~ ^[0-9a-f]{64}$ ]]; then
      pass_gate "paint-type_chora.deed canon digests present ($csha)"
    else
      fail_gate "paint-type_chora.deed has invalid canon SHA-256 digest(s)"
    fi
  fi

  if [ -f coordination.k9 ] && [ -f scripts/validate-coordination-k9.js ]; then
    if command -v bun >/dev/null 2>&1; then
      if bun scripts/validate-coordination-k9.js coordination.k9 >/dev/null; then
        pass_gate "coordination.k9 validated via Bun (ABNF v1.2)"
      else
        fail_gate "coordination.k9 failed ABNF v1.2 grammar validation"
      fi
    else
      warn_gate "bun not on PATH; skipped local coordination.k9 check"
    fi
  fi
}

# Gate 6: Warn if newly modified source/script files contain undocumented functions.
check_function_docstrings() {
  local changed="$1"
  local file undoc=0

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    [ -f "$file" ] || continue
    case "$file" in
      third_party/*|tests/*) continue ;;
      *.sh)
        if awk '
          /^[[:space:]]*#/ { doc = 1; next }
          /^[[:space:]]*$/ { doc = 0; next }
          /^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\(\)[[:space:]]*\{/ {
            if (!doc) {
              printf "  %s:%d undocumented shell function: %s\n", FILENAME, NR, $1 > "/dev/stderr"
              bad = 1
            }
          }
          { doc = 0 }
          END { exit(bad ? 1 : 0) }
        ' "$file"; then
          :
        else
          undoc=$((undoc + 1))
        fi
        ;;
      *.js)
        if awk '
          /^[[:space:]]*(\/\*\*|\*|\/\/)/ { doc = 1; next }
          /^[[:space:]]*$/ { doc = 0; next }
          /^[[:space:]]*(export[[:space:]]+)?(async[[:space:]]+)?function[[:space:]]+[A-Za-z_]/ {
            if (!doc) {
              printf "  %s:%d undocumented JS function\n", FILENAME, NR > "/dev/stderr"
              bad = 1
            }
          }
          { doc = 0 }
          END { exit(bad ? 1 : 0) }
        ' "$file"; then
          :
        else
          undoc=$((undoc + 1))
        fi
        ;;
    esac
  done <<< "$changed"

  if [ "$undoc" -gt 0 ]; then
    warn_gate "$undoc modified script file(s) contain functions without doc comments above them"
  else
    pass_gate "function docstrings verified across modified scripts"
  fi
}

CHANGED_FILES="$(collect_changed_files)"
ADDED_FILES="$(collect_added_files)"

check_actions_lock
check_changed_workflows "$CHANGED_FILES"
check_language_and_freeze_policies "$CHANGED_FILES" "$ADDED_FILES"
check_invisible_unicode "$CHANGED_FILES"
check_deed_and_k9
check_function_docstrings "$CHANGED_FILES"

if [ "$ERRORS" -gt 0 ]; then
  printf '[preflight] BLOCKED: %d error(s), %d warning(s)\n' "$ERRORS" "$WARNINGS" >&2
  exit 1
fi

printf '[preflight] READY: 0 errors, %d warning(s)\n' "$WARNINGS"
