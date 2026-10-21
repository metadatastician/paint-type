#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# verdict_grids_conformance_test.sh — end-to-end verification suite for:
#   Matrix 1: DEED (`paint-type_chora.deed`), K9 v1.0.0 contractiles (`*.k9.ncl`),
#             `coordination.k9` ABNF v1.2.0 (`validate-coordination-k9.js` via Bun),
#             and K9 Conformance Verdict Grid (`k9-grid.sh`).
#   Matrix 2: `.gitbot-fleet/enrolment.json`, `hypatia-scan.yml`,
#             `static-analysis-gate.yml` (`panic-attack` + `hypatia` + `pons-asinorum`
#             + `patch-bridge` -> unified findings schema 1.2.0), and Hypatia
#             Verdict Grid rendering (`verdict-grid.sh`).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

PASS=0
FAIL=0
pass() { printf '  PASS: %s\n' "$1"; PASS=$((PASS + 1)); }
fail() { printf '  FAIL: %s\n' "$1" >&2; FAIL=$((FAIL + 1)); }

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/verdict-grids-test-XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

# Locate or shallow-clone hyperpolymath/standards at the pinned commit if needed
STANDARDS_PIN="01c9b0c9f2646ebea0b30a9588462642ac94aee9"
STANDARDS_DIR="${STANDARDS_DIR:-/tmp/estate/standards}"
if [ ! -d "$STANDARDS_DIR/1-formats" ]; then
  STANDARDS_DIR="$TMP_DIR/standards"
  git init -q "$STANDARDS_DIR"
  git -C "$STANDARDS_DIR" remote add origin https://github.com/hyperpolymath/standards.git
  git -C "$STANDARDS_DIR" fetch -q --depth=1 origin "$STANDARDS_PIN"
  git -C "$STANDARDS_DIR" checkout -q FETCH_HEAD
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  paint-type — Matrix 1 & Matrix 2 Conformance Test Suite"
echo "═══════════════════════════════════════════════════════════════"

# ─── 1. DEED Conformance (`paint-type_chora.deed` + `deed_lint.js` + `deed-field.sh`) ───
echo ""
echo "1. DEED Conformance & Canon Lockstep"
DEED_FILE="$(bash scripts/deed-field.sh --find .)"
if [ "$DEED_FILE" = "./paint-type_chora.deed" ]; then
  pass "Single canonical repo deed discovered ($DEED_FILE)"
else
  fail "Expected ./paint-type_chora.deed, got '$DEED_FILE'"
fi

if bun "$STANDARDS_DIR/1-formats/deed/tools/deed_lint.js" "$DEED_FILE" > "$TMP_DIR/deed-lint.out" 2>&1; then
  pass "deed_lint.js passed on $DEED_FILE via Bun"
else
  cat "$TMP_DIR/deed-lint.out" >&2
  fail "deed_lint.js failed on $DEED_FILE"
fi

for clause in canon identity clade forges lineage status maturity ecosystem meta agentic manifest contractiles ply anchor neurosym playbook ensaid bot-directives integrations policies rsr-profile; do
  if bash scripts/deed-field.sh --has "$DEED_FILE" "$clause"; then
    pass "paint-type_chora.deed contains required ($clause) clause"
  else
    fail "paint-type_chora.deed missing ($clause) clause"
  fi
done

want_version="$(sed -n '/^\[canon\]/,/^\[/s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$STANDARDS_DIR/canon.lock" | head -n1)"
want_criteria="$(sed -n '/^criteria[[:space:]]*=/,/}/s/.*sha256[[:space:]]*=[[:space:]]*"\([0-9a-f]*\)".*/\1/p' "$STANDARDS_DIR/canon.lock" | head -n1)"
want_gates="$(sed -n '/^gates[[:space:]]*=/,/}/s/.*sha256[[:space:]]*=[[:space:]]*"\([0-9a-f]*\)".*/\1/p' "$STANDARDS_DIR/canon.lock" | head -n1)"

for pair in "version:$want_version" "criteria-sha256:$want_criteria" "gates-sha256:$want_gates"; do
  k="${pair%:*}"; want="${pair#*:}"
  got="$(bash scripts/deed-field.sh "$DEED_FILE" canon "$k")"
  if [ -n "$want" ] && [ "$got" = "$want" ]; then
    pass "canon :$k matches standards/canon.lock ($got)"
  else
    fail "canon :$k mismatch: got '$got', expected '$want'"
  fi
done

# ─── 2. K9 Coordination Nerve Receptor (`coordination.k9` ABNF v1.2.0 via Bun) ───
echo ""
echo "2. K9 Coordination Grammar (ABNF v1.2.0)"
if bun scripts/validate-coordination-k9.js coordination.k9 > "$TMP_DIR/ck9.out" 2>&1; then
  pass "coordination.k9 validated against coordination-k9-grammar_v1.2.abnf"
else
  cat "$TMP_DIR/ck9.out" >&2
  fail "coordination.k9 failed ABNF validation"
fi

# Negative control 1: YAML `---` separator (forbidden by ruling D318)
printf 'K9!\n# SPDX-License-Identifier: AGPL-3.0-or-later\n---\nmetadata:\n' > "$TMP_DIR/bad-d318.k9"
if ! bun scripts/validate-coordination-k9.js "$TMP_DIR/bad-d318.k9" >/dev/null 2>&1; then
  pass "Negative control: '---' document separator rejected (ruling D318)"
else
  fail "Negative control: '---' document separator was wrongly accepted"
fi

# Negative control 2: Out-of-order top-level sections
sed 's/^project:/do_not_create_dup:/' coordination.k9 > "$TMP_DIR/bad-section.k9"
if ! bun scripts/validate-coordination-k9.js "$TMP_DIR/bad-section.k9" >/dev/null 2>&1; then
  pass "Negative control: invalid section header rejected"
else
  fail "Negative control: invalid section header was wrongly accepted"
fi

# Also validate standards' own reference coordination.k9 to prove exact compatibility
if [ -f "$STANDARDS_DIR/coordination.k9" ]; then
  if bun scripts/validate-coordination-k9.js "$STANDARDS_DIR/coordination.k9" >/dev/null 2>&1; then
    pass "Cross-check: standards/coordination.k9 also passes validate-coordination-k9.js"
  else
    fail "Cross-check: standards/coordination.k9 failed validate-coordination-k9.js"
  fi
fi

# ─── 3. K9 Contractiles & Matrix 1 K9 Conformance Verdict Grid ───
echo ""
echo "3. K9 v1.0.0 Contractiles & Matrix 1 Verdict Grid"
K9_FILES=(
  "coordination.k9.ncl"
  ".machine_readable/contractiles/intend/intend.k9.ncl"
  ".machine_readable/contractiles/must/must.k9.ncl"
  ".machine_readable/contractiles/trust/trust.k9.ncl"
  ".machine_readable/contractiles/adjust/adjust.k9.ncl"
  ".machine_readable/contractiles/dust/dust.k9.ncl"
  ".machine_readable/contractiles/bust/bust.k9.ncl"
  "coordination.k9"
)

if bash "$STANDARDS_DIR/1-formats/k9/tools/k9-validate.sh" --layer L1 --json "${K9_FILES[@]}" > "$TMP_DIR/k9-corpus.json"; then
  pass "k9-validate.sh L0+L1 passed across all 7 *.k9.ncl files + coordination.k9"
else
  cat "$TMP_DIR/k9-corpus.json" >&2
  fail "k9-validate.sh L0+L1 reported errors"
fi

printf '%s\n' "${K9_FILES[@]}" > "$TMP_DIR/expected.txt"
printf 'typecheck ok\n' > "$TMP_DIR/typecheck.log"
bash "$STANDARDS_DIR/1-formats/k9/tools/k9-validate.sh" --self-test > "$TMP_DIR/selftest.log" 2>&1

bash "$STANDARDS_DIR/scripts/k9-grid.sh" render \
  --out "$TMP_DIR/k9-grid" \
  --results "$TMP_DIR/k9-corpus.json" \
  --expected "$TMP_DIR/expected.txt" \
  --require-files \
  --typecheck "$TMP_DIR/typecheck.log" --typecheck-code 0 \
  --selftest "$TMP_DIR/selftest.log" --selftest-code 0 \
  --corpus-code 0 \
  --repo "metadatastician/paint-type" \
  --sha "0000000000000000000000000000000000000000" \
  --run "1" \
  --url "https://github.com/metadatastician/paint-type/actions" \
  --canon-ref "$STANDARDS_PIN" >/dev/null

if grep -q '<!-- verdict-grid:k9 regenerated on every run -->' "$TMP_DIR/k9-grid/report.md" \
  && grep -q 'K9%20L0%E2%80%93L2-conformant-brightgreen' "$TMP_DIR/k9-grid/report.md" \
  && [ "$(jq -r .state "$TMP_DIR/k9-grid/report.json")" = "green" ] \
  && [ "$(jq -r .color "$TMP_DIR/k9-grid/badge.json")" = "brightgreen" ]; then
  pass "Matrix 1 K9 Verdict Grid rendered clean ('green' / 'brightgreen')"
else
  cat "$TMP_DIR/k9-grid/report.md" >&2
  fail "Matrix 1 K9 Verdict Grid did not grade 'green'"
fi

# ─── 4. Matrix 2 Hypatia Verdict Grid + Fleet Enrolment + Static Gate ───
echo ""
echo "4. Matrix 2 Hypatia Verdict Grid & Fleet Enrolment"
if jq -e '.schema == 1 and (.reporters | length) == 9 and .reporters.sustainabot.enrolled == false' .gitbot-fleet/enrolment.json >/dev/null; then
  pass ".gitbot-fleet/enrolment.json declares all 9 fleet reporters with schema v1"
else
  fail ".gitbot-fleet/enrolment.json invalid"
fi

mkdir -p "$TMP_DIR/findings"
printf '[]\n' > "$TMP_DIR/findings/findings.json"
printf '{"status":"ran","tool":"panic-attack","ref":"b689a74d2762a1c19839ae3fbd59b7fe5ef920ea","reason":null,"exit_code":0,"counts":{"critical":0,"high":0,"medium":0,"low":0},"report":{"weak_points":[],"statistics":{"critical_count":0},"taint_matrix":{"rows":[]}}}\n' > "$TMP_DIR/findings/panic.json"
printf '{"status":"ran","tool":"pons-asinorum","ref":"32e55677d8fd1f3010848dee22e822c2f385c2a0","reason":null,"exit_code":0,"counts":{"error":0,"warn":0,"info":0},"report":{"findings":[]}}\n' > "$TMP_DIR/findings/pons.json"

bash "$STANDARDS_DIR/scripts/verdict-grid.sh" render \
  --out "$TMP_DIR/hypatia-grid" \
  --hypatia "$TMP_DIR/findings/findings.json" \
  --panic "$TMP_DIR/findings/panic.json" \
  --pons "$TMP_DIR/findings/pons.json" \
  --enrolment .gitbot-fleet/enrolment.json \
  --report-backs .gitbot-fleet/report-backs \
  --repo "metadatastician/paint-type" \
  --sha "0000000000000000000000000000000000000000" \
  --run "1" \
  --url "https://github.com/metadatastician/paint-type/actions" \
  --hypatia-ref "aa34a4cc6ba39d873710ff39763fdb9cde715f9d" \
  --panic-ref "b689a74d2762a1c19839ae3fbd59b7fe5ef920ea" \
  --pons-ref "32e55677d8fd1f3010848dee22e822c2f385c2a0" >/dev/null

if grep -q '<!-- verdict-grid:hypatia regenerated on every run -->' "$TMP_DIR/hypatia-grid/report.md" \
  && grep -q 'Hypatia' "$TMP_DIR/hypatia-grid/report.md" \
  && grep -q 'panic-attack' "$TMP_DIR/hypatia-grid/report.md" \
  && grep -q 'pons asinorum' "$TMP_DIR/hypatia-grid/report.md" \
  && grep -q 'sustainabot' "$TMP_DIR/hypatia-grid/report.md" \
  && jq -e '.schemaVersion == 1' "$TMP_DIR/hypatia-grid/badge.json" >/dev/null; then
  pass "Matrix 2 Hypatia Verdict Grid rendered all 3 scanners + 9 fleet rows + badge.json"
else
  cat "$TMP_DIR/hypatia-grid/report.md" >&2
  fail "Matrix 2 Hypatia Verdict Grid missing expected rows"
fi

for wf in .github/workflows/dogfood-gate.yml .github/workflows/hypatia-scan.yml .github/workflows/governance.yml .github/workflows/scorecard.yml .github/workflows/secret-scanner.yml; do
  if grep -q "hyperpolymath/standards/.github/workflows/.*@$STANDARDS_PIN" "$wf"; then
    pass "$wf pinned to single canonical standards SHA ($STANDARDS_PIN)"
  else
    fail "$wf not pinned to $STANDARDS_PIN"
  fi
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Summary: $PASS passed, $FAIL failed"
echo "═══════════════════════════════════════════════════════════════"
test "$FAIL" -eq 0
