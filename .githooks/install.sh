#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Install repository git hooks by configuring `core.hooksPath = .githooks`.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  printf '[hooks] ERROR: not inside a git work tree: %s\n' "$ROOT" >&2
  exit 1
fi

git -C "$ROOT" config core.hooksPath .githooks
chmod +x "$ROOT"/.githooks/install.sh \
         "$ROOT"/.githooks/pre-commit \
         "$ROOT"/.githooks/commit-msg \
         "$ROOT"/.githooks/pre-push \
         "$ROOT"/scripts/preflight.sh \
         "$ROOT"/scripts/git-push-signed.sh

printf '[hooks] Installed core.hooksPath=.githooks for %s\n' "$ROOT"
