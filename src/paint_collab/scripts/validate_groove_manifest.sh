#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# Validate the committed paint-type Groove manifest against the v1 Groove
# manifest schema constraints (canonical schema:
# gossamer/schema/groove-manifest.schema.json). Uses jq only — no network, no
# extra deps. Run from anywhere:
#   bash src/paint_collab/scripts/validate_groove_manifest.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
MANIFEST="$PROJECT_DIR/.well-known/groove/manifest.json"

fail() { echo "  FAIL: $*" >&2; exit 1; }
ok()   { echo "  ok: $*"; }

command -v jq >/dev/null 2>&1 || fail "jq is required"
[ -f "$MANIFEST" ] || fail "missing $MANIFEST"
jq -e . "$MANIFEST" >/dev/null 2>&1 || fail "manifest is not valid JSON"
ok "valid JSON"

# Required top-level fields (schema: required groove_version, service_id, capabilities).
jq -e 'has("groove_version") and has("service_id") and has("capabilities")' "$MANIFEST" >/dev/null \
    || fail "missing a required field (groove_version / service_id / capabilities)"
ok "required fields present"

# groove_version const "1".
[ "$(jq -r '.groove_version' "$MANIFEST")" = "1" ] || fail "groove_version must be \"1\""
ok "groove_version == 1"

# service_id pattern ^[a-z][a-z0-9_-]*$
SID="$(jq -r '.service_id' "$MANIFEST")"
[[ "$SID" =~ ^[a-z][a-z0-9_-]*$ ]] || fail "service_id '$SID' violates ^[a-z][a-z0-9_-]*\$"
ok "service_id '$SID' matches pattern"

# service_version semver-ish ^[0-9]+\.[0-9]+\.[0-9]+
SVER="$(jq -r '.service_version // ""' "$MANIFEST")"
[[ "$SVER" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]] || fail "service_version '$SVER' not semver-ish"
ok "service_version '$SVER'"

# At least one capability.
[ "$(jq '.capabilities | length' "$MANIFEST")" -ge 1 ] || fail "no capabilities advertised"
ok "$(jq '.capabilities | length' "$MANIFEST") capability(ies)"

# Every capability.type is in the schema enum; every protocol is in its enum.
CAP_TYPES_ENUM='["voice","text","presence","spatial-audio","recording","tts","stt","integrity","feed-verification","hash-chain","attestation","octad-storage","drift-detection","temporal-versioning","scanning","static-analysis","panel-ui","bot-orchestration","workflow","dns-verify","config-orchestration","theorem-proving","custom"]'
PROTO_ENUM='["webrtc","websocket","http","grpc","nntps","custom"]'

jq -e --argjson e "$CAP_TYPES_ENUM" '
  [.capabilities[] | .type] | all(. as $t | $e | index($t) != null)
' "$MANIFEST" >/dev/null || fail "a capability.type is not in the schema enum"
ok "all capability.type values in enum"

jq -e --argjson e "$PROTO_ENUM" '
  [.capabilities[] | select(has("protocol")) | .protocol]
  | all(. as $p | $e | index($p) != null)
' "$MANIFEST" >/dev/null || fail "a capability.protocol is not in the schema enum"
ok "all capability.protocol values in enum"

# applicability values constrained to the schema enum.
jq -e '
  (.applicability // [])
  | all(. == "individual" or . == "team" or . == "massive-open")
' "$MANIFEST" >/dev/null || fail "applicability has a value outside the enum"
ok "applicability values in enum"

echo "Groove manifest OK: $MANIFEST"
