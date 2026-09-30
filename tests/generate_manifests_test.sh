#!/usr/bin/env bash

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/generate-manifests-test-XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

run_case() {
    local root="$TEST_ROOT/$1"
    mkdir -p "$root/scripts" "$root/content with spaces" "$root/target/nested"
    cp "$PROJECT_DIR/scripts/generate-manifests.sh" "$root/scripts/"

    bash "$root/scripts/generate-manifests.sh" --dry-run > "$TEST_ROOT/dry-run.log"
    grep -Fq "[DRY RUN] Would create: $root/content with spaces/0.1-AI-MANIFEST.a2ml" \
        "$TEST_ROOT/dry-run.log" || fail "dry-run did not preserve the literal path"
    [ ! -e "$root/content with spaces/0.1-AI-MANIFEST.a2ml" ] \
        || fail "dry-run created a manifest"
    [ ! -e "$root/content with spaces/README.adoc" ] \
        || fail "dry-run created a README"

    bash "$root/scripts/generate-manifests.sh" > "$TEST_ROOT/generate.log"
    [ -f "$root/content with spaces/0.1-AI-MANIFEST.a2ml" ] \
        || fail "manifest missing"
    [ -f "$root/content with spaces/README.adoc" ] || fail "README missing"
    grep -Fq 'content with spaces/' "$root/content with spaces/0.1-AI-MANIFEST.a2ml" \
        || fail "manifest content lost the directory name"
    [ ! -e "$root/0-AI-MANIFEST.a2ml" ] || fail "default mode generated the root manifest"
    [ ! -e "$root/target/nested/0.2-AI-MANIFEST.a2ml" ] || fail "pruned content was visited"
    [ ! -e "$root/target/nested/README.adoc" ] || fail "pruned README was created"
    [ ! -e "$root/injected-dollar" ] || fail "dollar command substitution executed"
    [ ! -e "$root/injected-backtick" ] || fail "backtick command substitution executed"

    cp "$root/content with spaces/0.1-AI-MANIFEST.a2ml" "$TEST_ROOT/expected-manifest"
    cp "$root/content with spaces/README.adoc" "$TEST_ROOT/expected-readme"
    bash "$root/scripts/generate-manifests.sh" > "$TEST_ROOT/second-run.log"
    cmp "$TEST_ROOT/expected-manifest" "$root/content with spaces/0.1-AI-MANIFEST.a2ml"
    cmp "$TEST_ROOT/expected-readme" "$root/content with spaces/README.adoc"
}

run_case normal
run_case 'repo $(touch injected-dollar) `touch injected-backtick` "quoted" space'
printf 'PASS: literal paths, command substitution safety, dry-run, pruning and existing files\n'
