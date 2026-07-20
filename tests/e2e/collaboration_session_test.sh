#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# E2E Test: Collaboration session (v0.5.0, issue #15; TEST-NEEDS P3)
#
# Drives the full two-peer collaboration scenario over the in-process
# simulated transport:
#   discovery (Groove) → permission grant → concurrent paint →
#   reordered + duplicated op exchange → CRDT convergence →
#   permission-denied paint (loud, not silent) → off-by-default LLM gate.
#
# The live, latency-measured WebRTC variant (Burble transport, <10ms p95)
# requires a running Burble bridge + browser WebRTC stack and is out of scope
# for CI; this scenario validates the collaboration *semantics* deterministically.
#
# Usage:  bash tests/e2e/collaboration_session_test.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CRATE_DIR="$PROJECT_DIR/src/paint_collab"

GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
log()  { echo -e "${BLUE}→${NC} $*"; }
pass() { echo -e "${GREEN}✓${NC} $*"; }
err()  { echo -e "${RED}✗${NC} $*" >&2; }

if ! command -v cargo >/dev/null 2>&1; then
    err "cargo not found — cannot run the collaboration session test"
    exit 127
fi

log "Running two-peer collaboration scenario (paint_collab::tests::e2e_two_peer)"
if cargo test --manifest-path "$CRATE_DIR/Cargo.toml" --test e2e_two_peer -- --nocapture; then
    pass "two-peer session converges; permission + LLM gates enforced"
else
    err "collaboration session test FAILED"
    exit 1
fi

log "Running CRDT convergence property tests (CONC-1/CONC-2 ground truth)"
if cargo test --manifest-path "$CRATE_DIR/Cargo.toml" --test convergence; then
    pass "CRDT merge commutative + associative + idempotent over random permutations"
else
    err "convergence property tests FAILED"
    exit 1
fi

pass "Collaboration session E2E complete"
