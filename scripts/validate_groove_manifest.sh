#!/usr/bin/env bash
# Validate Groove capability discovery manifests (.well-known/groove/manifest.json
# and www/.well-known/groove/manifest.json) for JSON validity, required Groove v1
# fields, and exact parity.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ROOT_MANIFEST="$REPO_ROOT/.well-known/groove/manifest.json"
WWW_MANIFEST="$REPO_ROOT/www/.well-known/groove/manifest.json"

for f in "$ROOT_MANIFEST" "$WWW_MANIFEST"; do
    if [ ! -f "$f" ]; then
        echo "FAIL: missing Groove manifest: $f" >&2
        exit 1
    fi
done

if ! command -v jq >/dev/null 2>&1; then
    echo "FAIL: jq is required to validate Groove manifests" >&2
    exit 1
fi

jq -e '
    type == "object" and
    (.groove_version == "1") and
    (.service_id == "paint-type") and
    (.service_version | type == "string" and length > 0) and
    (.capabilities | type == "object" and has("collab-canvas") and has("edit-stream")) and
    (.consumes | type == "array")
' "$ROOT_MANIFEST" >/dev/null

if ! cmp -s "$ROOT_MANIFEST" "$WWW_MANIFEST"; then
    echo "FAIL: $ROOT_MANIFEST and $WWW_MANIFEST have drifted" >&2
    diff -u "$ROOT_MANIFEST" "$WWW_MANIFEST" >&2 || true
    exit 1
fi

echo "PASS: Groove manifests valid and synchronized"
